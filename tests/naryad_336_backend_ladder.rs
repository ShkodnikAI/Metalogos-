// ── Наряд №336 (P1, feature/registry): BackendSelect — the backend
//    ladder and Degraded(t), typed degradation (ADR-0165) ─────────────
//
// Red/green corpus:
//   (1) the ladder picks the FIRST available rung (mock mode, both
//       backends) and the mode is visible in the result — never hidden;
//   (2) exhaustion → Degraded(t): ok=false, class preserved, the stable
//       code BACKEND_DEGRADED, every rung attempted (the attempts list
//       IS the per-rung audit surface, №326 posture) — no panic, no
//       silent mock, the program keeps running;
//   (3) real mode NEVER returns a mock: rungs without verified weights
//       are unavailable and the ladder honestly exhausts (the loud
//       mock boundary, ADR-0165 §2.3);
//   (4) shape errors (unknown class / empty ladder / duplicate rungs)
//       are loud Errs — catchable by try (ADR-0142);
//   (5) the static companion check (ADR-0165 §2.4): unknown rung, class
//       mismatch, unknown class word, duplicate rungs, and the
//       production profile + PendingNo334 rung are COMPILE errors;
//       production + pinned rungs compile; non-literal ladders are not
//       statically verified (no false positives).

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

// ── (1) The first available rung wins; the mode is visible ──────────

const SELECT_PROG: &str = r#"
pattern Pick(_tick: String) -> String {
  let sel = backend_select("omni", ["nemotron-omni", "wall-oss"])
  if sel.ok {
    return sel.backend + "/" + sel.mode + "/" + sel.weights_id
  }
  return "unexpected"
}
flow Main { input: String = "t" -> Pick -> output }
"#;

#[test]
fn ladder_selects_first_available_rung_both_backends() {
    // Mock mode is the DEFAULT (unset env). Env-locked: the real-mode
    // test mutates the process-global flag (the №334 test discipline).
    // The ladder is statically verified (both rungs are omni-class);
    // priority wins: the FIRST rung is selected, the second is never
    // tried (a selection, not a broadcast).
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");
    let expected = "nemotron-omni/mock/nemotron-3-nano-omni-30b-a3b";
    let out = run_tw(SELECT_PROG).expect("tw runs");
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), expected);
    let out_vm = run_vm(SELECT_PROG).expect("vm runs");
    assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), expected);
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (2) Exhaustion → Degraded(t): typed, loud, non-fatal ────────────

const DEGRADE_PROG: &str = r#"
pattern Ghost(_x: String) -> String {
  return "no-such-backend"
}
pattern WrongClass(_x: String) -> String {
  return "chatterbox"
}
pattern Degrade(_tick: String) -> String {
  // A runtime-built ladder: statically unverifiable, so the runtime
  // checks own it — 'no-such-backend' has no registry record,
  // 'chatterbox' is tts while the ladder serves omni. Both attempts
  // are audited; exhaustion → Degraded(omni).
  let words = [Ghost("a"), WrongClass("b")]
  let sel = backend_select("omni", words)
  if sel.ok {
    return "unexpected: a failed ladder selected a rung"
  }
  if sel.class == "omni" {
    if sel.error.code == "BACKEND_DEGRADED" {
      let n = len(sel.attempts)
      let first = sel.attempts[0]
      let second = sel.attempts[1]
      if first.status == "unavailable" {
        if second.status == "unavailable" {
          return "degraded/" + sel.class + "/" + to_string(n) +
                 "/" + first.backend + "/" + second.backend
        }
      }
    }
  }
  return "degraded-shape-broken"
}
flow Main { input: String = "t" -> Degrade -> output }
"#;

#[test]
fn exhaustion_returns_degraded_not_a_panic_not_a_mock() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");
    let expected = "degraded/omni/2/no-such-backend/chatterbox";
    let out = run_tw(DEGRADE_PROG).expect("degraded program KEEPS RUNNING (no panic)");
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), expected);
    let out_vm = run_vm(DEGRADE_PROG).expect("vm parity for Degraded");
    assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), expected);
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (3) Real mode never substitutes a mock (the loud boundary) ──────

#[test]
fn real_mode_exhausts_honestly_never_a_mock() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_LLM_MOCK", "false");
    // The ladder's real-mode rungs are unavailable without SHA-verified
    // weights on disk (PARKED №294): the result must be Degraded —
    // ok=false — and MUST NOT carry mode "mock".
    let src = r#"
pattern R(_t: String) -> String {
  let sel = backend_select("omni", ["nemotron-omni", "wall-oss"])
  if sel.ok {
    return "MOCK-LEAK:" + sel.backend
  }
  if sel.class == "omni" {
    return "degraded/omni/" + to_string(len(sel.attempts))
  }
  return "shape-broken"
}
flow Main { input: String = "t" -> R -> output }
"#;
    let out = run_tw(src).expect("real-mode ladder runs and degrades");
    let got = out.as_deref().unwrap_or_default().trim_end().to_string();
    assert_eq!(got, "degraded/omni/2", "got: {}", got);
    assert!(
        !got.contains("MOCK-LEAK"),
        "real mode must never return a mock"
    );
    let out_vm = run_vm(src).expect("vm real-mode parity");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        "degraded/omni/2"
    );
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (4) Shape errors are loud Errs — catchable by try (ADR-0142) ────

#[test]
fn shape_errors_are_loud_and_try_catchable() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");
    // Unknown class word: a runtime Err (catchable), NOT a Degraded —
    // the ladder contract only covers registry-verified rungs. A
    // dynamic (non-literal) class word keeps the static check out of
    // the way so the RUNTIME path is what's exercised here.
    let src = r#"
pattern Bad(_x: String) -> String {
  return "nosuchclass"
}
pattern T(_t: String) -> String {
  let c = Bad("a")
  let r = try backend_select(c, ["whisper-turbo"])
  if r.ok == false {
    return "caught/" + r.error.code
  }
  return "not-caught"
}
flow Main { input: String = "t" -> T -> output }
"#;
    let out = run_tw(src).expect("try-catch runs");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "caught/RUNTIME_ERROR"
    );
    // Without try: the same call is a loud runtime error on both backends.
    // (Field access needs a binding — `.ok` on a bare call is not mlog
    // surface syntax; bind, then read.)
    let raw = r#"
pattern Bad2(_x: String) -> String {
  return "nosuchclass"
}
pattern U(_t: String) -> String {
  let c = Bad2("a")
  let sel = backend_select(c, ["whisper-turbo"])
  return to_string(sel.ok)
}
flow Main { input: String = "t" -> U -> output }
"#;
    let err = run_tw(raw).expect_err("unknown class must be a loud runtime error");
    assert!(err.contains("unknown backend class"), "got: {}", err);
    let err_vm = run_vm(raw).expect_err("vm: unknown class is loud too");
    assert!(err_vm.contains("unknown backend class"), "got: {}", err_vm);
    // Empty ladder and duplicate rungs: the silent-fallback refusals.
    // Both ladders are runtime values (var-bound), so the runtime owns
    // them; the static check fires only for literal call-site ladders.
    let empty = r#"
pattern None_() -> List {
  return []
}
pattern V(_t: String) -> String {
  let words = None_()
  let sel = backend_select("stt", words)
  return "x"
}
flow Main { input: String = "t" -> V -> output }
"#;
    let e = run_tw(empty).expect_err("empty ladder is a runtime refusal");
    assert!(e.contains("ladder is EMPTY"), "got: {}", e);
    let dupe = r#"
pattern W(_t: String) -> String {
  let words = ["whisper-turbo", "whisper-turbo"]
  let sel = backend_select("stt", words)
  return "x"
}
flow Main { input: String = "t" -> W -> output }
"#;
    let e = run_tw(dupe).expect_err("duplicate rung is a runtime refusal");
    assert!(e.contains("duplicate ladder rung"), "got: {}", e);
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (5) The static companion check — build-time ladder verification ─

#[test]
fn companion_check_unknown_rung_and_class_mismatch_are_compile_errors() {
    let unknown_rung = r#"
pattern P1(_x: String) -> String {
  let sel = backend_select("stt", ["whisper-turbo", "nope"])
  return sel.backend
}
flow Main { input: String = "t" -> P1 -> output }
"#;
    let e = compile_err(unknown_rung);
    assert!(
        e.contains("ladder rung 'nope' has no registry record"),
        "got: {}",
        e
    );
    let mismatch = r#"
pattern P2(_x: String) -> String {
  let sel = backend_select("omni", ["chatterbox"])
  return sel.backend
}
flow Main { input: String = "t" -> P2 -> output }
"#;
    let e = compile_err(mismatch);
    assert!(
        e.contains("is class 'tts', ladder serves 'omni'"),
        "got: {}",
        e
    );
    let unknown_class = r#"
pattern P3(_x: String) -> String {
  let sel = backend_select("flying", ["whisper-turbo"])
  return sel.backend
}
flow Main { input: String = "t" -> P3 -> output }
"#;
    let e = compile_err(unknown_class);
    assert!(e.contains("unknown backend class 'flying'"), "got: {}", e);
    let dupe = r#"
pattern P4(_x: String) -> String {
  let sel = backend_select("stt", ["whisper-turbo", "whisper-turbo"])
  return sel.backend
}
flow Main { input: String = "t" -> P4 -> output }
"#;
    let e = compile_err(dupe);
    assert!(e.contains("duplicate ladder rung"), "got: {}", e);
}

#[test]
fn device_production_profile_refuses_unverifiable_rungs_at_build_time() {
    // wall-oss is Restrictive-license AND PendingNo334 — unverifiable
    // for a production device profile: COMPILE error (ADR-0165 §2.4).
    let bad = r#"
profile device { mode: production }
pattern P5(_x: String) -> String {
  let sel = backend_select("omni", ["wall-oss", "nemotron-omni"])
  return sel.backend
}
flow Main { input: String = "t" -> P5 -> output }
"#;
    let e = compile_err(bad);
    assert!(
        e.contains("UNVERIFIABLE for device profile production"),
        "got: {}",
        e
    );
    // The pinned rung is fine in production: only the pending one is
    // named by the error.
    assert!(e.contains("wall-oss"), "got: {}", e);
    assert!(
        !e.contains("nemotron-omni"),
        "pinned rung must not be flagged: {}",
        e
    );
    // The same ladder compiles under development (no device profile).
    let dev = r#"
pattern P6(_x: String) -> String {
  let sel = backend_select("omni", ["wall-oss", "nemotron-omni"])
  return sel.backend
}
flow Main { input: String = "t" -> P6 -> output }
"#;
    assert!(
        metalogos::compile_program(dev.trim()).is_ok(),
        "development (default) has no static ladder constraints"
    );
    // A fully pinned ladder compiles under production.
    let pinned = r#"
profile device { mode: production }
pattern P7(_x: String) -> String {
  let sel = backend_select("stt", ["whisper-turbo"])
  return sel.backend
}
flow Main { input: String = "t" -> P7 -> output }
"#;
    assert!(
        metalogos::compile_program(pinned.trim()).is_ok(),
        "pinned rungs are production-verifiable"
    );
    // Unknown device-mode word is a loud profile-shape error (№325 rule).
    let wrong_mode = r#"
profile device { mode: edge }
pattern P8(_x: String) -> String {
  return "x"
}
flow Main { input: String = "t" -> P8 -> output }
"#;
    let e = compile_err(wrong_mode);
    assert!(
        e.contains("unknown mode") && e.contains("production, development"),
        "got: {}",
        e
    );
}

#[test]
fn non_literal_ladder_is_not_statically_verified() {
    // A runtime-built ladder can't be checked statically — the runtime
    // checks own it. No false-positive compile error (ADR-0165 §2.4).
    let src = r#"
pattern Build(_x: String) -> String {
  return "kokoro"
}
pattern P9(_x: String) -> String {
  let words = [Build("a")]
  let sel = backend_select("stt", words)
  return to_string(sel.ok)
}
flow Main { input: String = "t" -> P9 -> output }
"#;
    assert!(
        metalogos::compile_program(src.trim()).is_ok(),
        "non-literal ladders are runtime-owned, not compile errors"
    );
}

// ── (6) The golden example: the full cycle on both backends ─────────

#[test]
fn w1_degrade_example_matches_expected_on_both_backends() {
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/w1_degrade.mlog"),
    )
    .expect("example exists");
    let expected = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/w1_degrade.expected"),
    )
    .expect("expected file exists");
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");
    let out = run_tw(&src).expect("example runs on tw");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        expected.trim_end()
    );
    let out_vm = run_vm(&src).expect("example runs on vm");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        expected.trim_end()
    );
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (7) No-stubs discipline (№16.0-D) ────────────────────────────────

#[test]
fn no_stubs_in_the_n336_surface() {
    for file in [
        "src/builtins/backends.rs",
        "src/profile.rs",
        "src/semantic.rs",
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
        let content = std::fs::read_to_string(&path).expect(file);
        for bad in ["todo!", "unimplemented!", "SKELETON"] {
            assert!(
                !content.contains(bad),
                "{} contains {} — stubs are forbidden (№16.0-D)",
                file,
                bad
            );
        }
    }
}
