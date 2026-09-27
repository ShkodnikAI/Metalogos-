// ── Naryad #478 (P1, llm/security): the mock unification onto the №454 SSOT ──
//
// Contract (issue #726, audit 26.09 §3.6 + the 27.09 fact-check): the
// retired `METALOGOS_LLM_MOCK` env was re-read in 7 places with the old
// default-ON semantics. All sites now route through
// `crate::llm::mock_llm_requested()` (№454): the deterministic mock
// answers ONLY when `METALOGOS_MOCK_LLM=1|true` is explicit; the default
// is the REAL path, which refuses LOUD — for the media surfaces naming
// the weights and the №294 PARKED boundary, for call_llm naming the
// missing provider key. Never a silent stub-substitution.
//
// This file pins the two surfaces whose loud real-path refusal had NO
// dedicated pin before №478 (the office human surface and the bare
// call_llm surface); the media surfaces (video/ocr/vision/voice) are
// pinned by the №334/№336/№407 contract suites, re-pinned to the
// explicit-mock contract in the same naryad.

use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
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

#[test]
fn n478_call_llm_defaults_to_the_loud_real_path() {
    let _env = lock_env();
    // The mock opt-in must be ABSENT: the real path is the default (№454).
    std::env::remove_var("METALOGOS_MOCK_LLM");
    // №401 hygiene: a real network call must never fire in tests — the
    // provider key is REMOVED (an empty string would still be treated as
    // present and would leak a real HTTP request).
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");

    let src = r#"
pattern Ask(question: String) -> String {
  return call_llm("Answer:", question)
}
flow Main {
  input: String = "hi" -> Ask -> output
}
"#;
    let err = metalogos::run_program(src)
        .expect_err("call_llm without the mock opt-in must hit the loud real path");
    assert!(
        err.contains("METALOGOS_API_KEY"),
        "the real-path refusal must name the missing provider key (never a silent mock), got: {}",
        err
    );
    assert!(
        !err.contains("[MOCK:"),
        "the output must NOT contain a mock answer: {}",
        err
    );

    // The explicit opt-in restores the deterministic mock.
    let _mock = EnvVar::set("METALOGOS_MOCK_LLM", "1");
    let out = metalogos::run_program(src).expect("call_llm with the explicit mock opt-in answers");
    assert!(
        out.unwrap_or_default().contains("[MOCK:"),
        "the explicit opt-in must produce the deterministic mock",
    );
}

#[test]
fn n478_office_human_defaults_to_the_loud_real_path() {
    let _env = lock_env();
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");

    let src = r#"
pattern Talk(_x: String) -> String {
  let _p = human_create("n478_probe", "helpful tester")
  return human_respond("n478_probe", "hello there")
}
flow Main {
  input: String = "go" -> Talk -> output
}
"#;
    let err = metalogos::run_program(src)
        .expect_err("human_respond without the mock opt-in must hit the loud real path");
    let joined = err.clone();
    assert!(
        joined.contains("METALOGOS_API_KEY") || joined.contains("requires its weights"),
        "the office-human real-path refusal must be loud (the provider key or the weights boundary), got: {}",
        joined
    );
    assert!(
        !joined.contains("[n478_probe"),
        "the output must NOT contain a silent mock answer: {}",
        joined
    );
}
