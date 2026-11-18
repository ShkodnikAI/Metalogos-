// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── Наряд №973 (P1, domain/security; issue #1972; ADR-0182 §2b/§3.2) ──
//
// The capability axis of `backend_select` — the §2b residue of the
// media-input bridge (steps 1–3 landed by №599/№938/№939; the inputs
// descriptors by №599/№941/№945): the OPTIONAL third argument names the
// media input the call needs (`image` | `audio` | `video_frame` |
// `video_segment`), and a rung whose registry entry does not DECLARE the
// capability is unavailable — the ladder descends, it never steps
// sideways into an unsatisfied capability (the fail-closed selection
// rule of §3.2); exhaustion is the typed Degraded with the capability
// named in the message; an unknown capability word is a loud Err before
// any mode resolution.
//
// Red/green corpus:
//   (1) a satisfied capability selects the entry (both backends);
//   (2) an unsatisfied capability exhausts the ladder into the typed
//       Degraded — the message names the capability, every attempt
//       carries the capability-mismatch reason (both backends);
//   (3) the 2-arg golden form is byte-identical in behavior (both
//       backends);
//   (4) an unknown capability word refuses LOUDLY (the Err carries the
//       accepted-words list);
//   (5) the arity/shape refusals: 4 arguments and a non-String
//       capability are Errs (the same loud posture).
//
// Mutation verification (the №382 protocol): dropping the capability
// check from the walk turns (2) red (a rung selects without the
// capability); dropping the capability-word parse turns (4) red (an
// unknown word silently no-filters); reverting the arity to 2 turns (5)
// red.

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

/// The deterministic mock contract (№478/№454): explicit per-test opt-in,
/// removed after — the mode is never inherited by a later test.
fn with_mock(f: impl FnOnce()) {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    f();
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

// ── (1) A satisfied capability selects the entry ─────────────────────
// molmoact2 (vision-understanding) declares [Image] (№599) — selecting
// with the "image" capability satisfies the rung.

const SATISFIED: &str = r#"
pattern Sel(_tick: String) -> String {
  let words = ["molmoact2"]
  let sel = backend_select("vision-understanding", words, "image")
  if sel.ok {
    return "ok/" + sel.backend + "/" + sel.mode
  }
  return "unexpected-degraded"
}
flow Main { input: String = "tick" -> Sel -> output }
"#;

#[test]
fn capability_satisfied_selects_on_both_backends() {
    with_mock(|| {
        let expected = "ok/molmoact2/mock";
        let out = run_tw(SATISFIED).expect("tw runs");
        assert_eq!(out.as_deref().unwrap_or_default().trim_end(), expected);
        let out_vm = run_vm(SATISFIED).expect("vm runs");
        assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), expected);
    });
}

// ── (2) An unsatisfied capability exhausts into the typed Degraded ───
// Both vision-understanding entries declare [Image] only; selecting with
// "audio" makes every rung unavailable — the attempts trail carries the
// capability-mismatch reason, the message names the capability (§3.2:
// the ladder descends, it never steps sideways).

const UNSATISFIED: &str = r#"
pattern Degrade(_tick: String) -> String {
  let words = ["z-image-turbo", "molmoact2"]
  let sel = backend_select("vision-understanding", words, "audio")
  if sel.ok {
    return "unexpected-selected"
  }
  if sel.error.code == "BACKEND_DEGRADED" {
    let first = sel.attempts[0]
    let second = sel.attempts[1]
    if first.status == "unavailable" {
      if second.status == "unavailable" {
        if contains(first.reason, "capability mismatch") {
          if contains(second.reason, "capability mismatch") {
            return "degraded/" + sel.error.message
          }
        }
      }
    }
  }
  return "unexpected-shape"
}
flow Main { input: String = "tick" -> Degrade -> output }
"#;

#[test]
fn capability_unsatisfied_degrades_typed_on_both_backends() {
    with_mock(|| {
        let want = "degraded/backend ladder exhausted: every rung for class \
'vision-understanding' accepting 'audio' is unavailable (2 attempt(s) audited)";
        let out = run_tw(UNSATISFIED).expect("tw runs");
        assert_eq!(out.as_deref().unwrap_or_default().trim_end(), want);
        let out_vm = run_vm(UNSATISFIED).expect("vm runs");
        assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), want);
    });
}

// ── (3) The 2-arg golden form is untouched ───────────────────────────
// The №336 contract holds byte-for-byte: no capability named = no
// capability filtering (whisper-turbo, class stt, declares [Audio] —
// irrelevant when the selection names no capability).

const GOLDEN_TWO_ARG: &str = r#"
pattern Sel(_tick: String) -> String {
  let words = ["whisper-turbo"]
  let sel = backend_select("stt", words)
  if sel.ok {
    return "ok/" + sel.backend + "/" + sel.mode
  }
  return "unexpected-degraded"
}
flow Main { input: String = "tick" -> Sel -> output }
"#;

#[test]
fn two_arg_golden_form_unchanged_on_both_backends() {
    with_mock(|| {
        let expected = "ok/whisper-turbo/mock";
        let out = run_tw(GOLDEN_TWO_ARG).expect("tw runs");
        assert_eq!(out.as_deref().unwrap_or_default().trim_end(), expected);
        let out_vm = run_vm(GOLDEN_TWO_ARG).expect("vm runs");
        assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), expected);
    });
}

// ── (4) An unknown capability word refuses LOUDLY ────────────────────
// The word parse is the MediaKind slug vocabulary; an unknown word is an
// Err carrying the accepted words — never a silent no-filter (fail-closed,
// the loud-refusal discipline of §3.2).

const UNKNOWN_WORD: &str = r#"
pattern Sel(_tick: String) -> String {
  let words = ["whisper-turbo"]
  let sel = backend_select("stt", words, "png")
  return "selected/" + sel.backend
}
flow Main { input: String = "tick" -> Sel -> output }
"#;

#[test]
fn unknown_capability_word_refuses_loudly() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let err = run_tw(UNKNOWN_WORD).expect_err("the unknown word must refuse");
    assert!(
        err.contains("input capability must be one of"),
        "the refusal must name the accepted words, got: {err}"
    );
    let err_vm = run_vm(UNKNOWN_WORD).expect_err("the VM must refuse too");
    assert!(
        err_vm.contains("input capability must be one of"),
        "the VM refusal must name the accepted words, got: {err_vm}"
    );
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

// ── (5) The arity/shape refusals stay loud ───────────────────────────

const FOUR_ARGS: &str = r#"
pattern Sel(_tick: String) -> String {
  let words = ["whisper-turbo"]
  let sel = backend_select("stt", words, "audio", "extra")
  return "selected/" + sel.backend
}
flow Main { input: String = "tick" -> Sel -> output }
"#;

const NON_STRING_CAP: &str = r#"
pattern Sel(_tick: String) -> String {
  let words = ["whisper-turbo"]
  let sel = backend_select("stt", words, 42)
  return "selected/" + sel.backend
}
flow Main { input: String = "tick" -> Sel -> output }
"#;

#[test]
fn arity_and_shape_refusals_stay_loud() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("METALOGOS_MOCK_LLM");
    // The layered refusal surface (the №523 posture): the TW's static
    // arity check refuses at COMPILE time (the registry spec message);
    // the VM's runtime validator is the loud backstop (the builtin's own
    // message). Both refuse — the fragments differ per layer.
    for (name, prog, tw_fragment, vm_fragment) in [
        (
            "four-args",
            FOUR_ARGS,
            "function 'backend_select' expects 2..3 argument(s), got 4",
            "expects 2..3 arguments (class, ladder, input capability?), got 4",
        ),
        (
            "non-string-capability",
            NON_STRING_CAP,
            "input capability must be String",
            "input capability must be String",
        ),
    ] {
        let err = run_tw(prog).expect_err(&format!("{name}: must be an Err, got Ok"));
        assert!(
            err.contains(tw_fragment),
            "{name}: the refusal must carry '{tw_fragment}', got: {err}"
        );
        let err_vm = run_vm(prog).expect_err(&format!("{name}: the VM must refuse, got Ok"));
        assert!(
            err_vm.contains(vm_fragment),
            "{name}: the VM refusal must carry '{vm_fragment}', got: {err_vm}"
        );
    }
    std::env::remove_var("METALOGOS_MOCK_LLM");
}
