// ── Naryad #478 (P1, llm/security): the mock unification onto the №454 SSOT ──
// (re-pinned by №879, wave 78 — W-G, ADR-0208 §2.2/§5)
//
// Contract (issue #726, audit 26.09 §3.6 + the 27.09 fact-check): the
// retired `METALOGOS_LLM_MOCK` env was re-read in 7 places with the old
// default-ON semantics. All sites route through
// `crate::llm::mock_llm_requested()` (№454): the deterministic mock
// answers ONLY when `METALOGOS_MOCK_LLM=1|true` is explicit. Never a
// silent stub-substitution.
//
// The №879 re-pin (ADR-0208 §5 — "the ONE intentional posture change"):
// the DEFAULT composition (nothing declared — no mock flag, no
// provider/key/base-url env) is now the LOCAL-FIRST provider: the
// deterministic, non-echo, offline `[mock-llm:<hex>]` marker. The №454
// SECURITY essence is preserved and STRONGER here: the prompt material
// (instructions, recalled memory, history) NEVER reaches the caller in
// ANY mode — the local answer is VISIBLE as such (the frozen marker),
// never a silent stand-in for an external voice. The №454/№478
// fail-loud posture lives on the DECLARED EXTERNAL lane: an explicit
// provider declaration without credentials still refuses LOUD, naming
// the missing key.
//
// This file pins the two surfaces whose composition had no dedicated
// pin before №478 (the office human surface and the bare call_llm
// surface); the media surfaces (video/ocr/vision/voice) ride the
// `mock_llm_requested()` PREDICATE — untouched by №879 (their own
// lanes, ADR-0208 §3).

use std::sync::Mutex;

/// №879: serialize the whole file — the tests mutate the process-global
/// LLM composition env (the №251/№478 discipline).
static FILE_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

struct EnvVar(&'static str);
impl EnvVar {
    fn set(name: &'static str, value: &str) -> Self {
        std::env::set_var(name, value);
        EnvVar(name)
    }
}
impl Drop for EnvVar {
    fn drop(&mut self) {
        std::env::remove_var(self.0);
    }
}

/// №879/№401 hygiene: clear EVERY declaration surface — the local-first
/// mode is "nothing declared", and a leftover key would silently flip
/// the composition to the external lane.
fn clear_llm_env() {
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_LLM_PROVIDER");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("METALOGOS_OPENAI_BASE_URL");
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
}

#[test]
fn n478_call_llm_default_is_the_local_first_provider() {
    let _env = lock_env();
    clear_llm_env();
    let src = r#"
pattern Ask(question: String) -> String {
  return call_llm("Answer:", question)
}
flow Main {
  input: String = "hi" -> Ask -> output
}
"#;
    // (a) Nothing declared → the LOCAL provider (ADR-0208 §2.2): the
    // visible deterministic marker — never an echo, never the old loud
    // real-backend failure, never the explicit-mock [MOCK:] lane.
    let out = metalogos::run_program(src)
        .expect("call_llm with nothing declared answers on the local provider (№879)")
        .unwrap_or_default();
    assert!(
        out.starts_with("[mock-llm:") && out.ends_with("]"),
        "the local-first answer must be the visible deterministic marker, got: {}",
        out
    );
    assert!(
        !out.contains("[MOCK:"),
        "the [MOCK:] lane is the EXPLICIT opt-in only — got: {}",
        out
    );
    assert!(
        !out.contains("Answer:") && !out.contains("hi"),
        "the local answer reveals NOTHING about the prompt/input (№454) — got: {}",
        out
    );

    // (b) The DECLARED external lane keeps the №454 fail-loud posture:
    // an explicit provider declaration without credentials refuses
    // loudly, naming the missing key (never a silent mock).
    let _provider = EnvVar::set("METALOGOS_LLM_PROVIDER", "anthropic");
    let err = metalogos::run_program(src).expect_err(
        "a DECLARED external provider without credentials must refuse loudly (№454 preserved)",
    );
    drop(_provider);
    assert!(
        err.contains("METALOGOS_API_KEY"),
        "the declared-external refusal must name the missing provider key, got: {}",
        err
    );
    assert!(
        !err.contains("[MOCK:"),
        "the output must NOT contain a mock answer: {}",
        err
    );

    // (c) The explicit opt-in restores the deterministic mock (the
    // declared semantics keep their exact pre-№879 shape).
    let _mock = EnvVar::set("METALOGOS_MOCK_LLM", "1");
    let out = metalogos::run_program(src).expect("call_llm with the explicit mock opt-in answers");
    drop(_mock);
    assert!(
        out.unwrap_or_default().contains("[MOCK:"),
        "the explicit opt-in must produce the deterministic mock",
    );
}

#[test]
fn n478_office_human_default_is_the_local_first_provider() {
    let _env = lock_env();
    clear_llm_env();
    let src = r#"
pattern Talk(_x: String) -> String {
  let _p = human_create("n478_probe", "helpful tester")
  return human_respond("n478_probe", "hello there")
}
flow Main {
  input: String = "go" -> Talk -> output
}
"#;
    // (a) Nothing declared → the office-human surface answers on the
    // LOCAL provider: the visible deterministic marker — never the
    // silent persona stand-in of the mock branch.
    let out = metalogos::run_program(src)
        .expect("human_respond with nothing declared answers on the local provider (№879)")
        .unwrap_or_default();
    assert!(
        out.starts_with("[mock-llm:") && out.ends_with("]"),
        "the local-first human answer must be the visible marker, got: {}",
        out
    );
    assert!(
        !out.contains("[n478_probe"),
        "the output must NOT contain the silent mock persona answer: {}",
        out
    );

    // (b) The DECLARED external lane keeps the loud real-path refusal
    // (the provider key or the weights boundary).
    let _provider = EnvVar::set("METALOGOS_LLM_PROVIDER", "anthropic");
    let err = metalogos::run_program(src).expect_err(
        "human_respond on the declared-external lane without credentials refuses loudly",
    );
    drop(_provider);
    assert!(
        err.contains("METALOGOS_API_KEY") || err.contains("requires its weights"),
        "the office-human declared-external refusal must be loud (the provider key or the weights boundary), got: {}",
        err
    );
    assert!(
        !err.contains("[n478_probe"),
        "the output must NOT contain a silent mock answer: {}",
        err
    );
}
