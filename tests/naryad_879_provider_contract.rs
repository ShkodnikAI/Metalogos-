// ── tests/naryad_879_provider_contract.rs ────────────────────────────
// Наряд №879 (issue #1739, wave 78 — W-G, P1, [core][llm], ADR-0208):
// the provider-agnostic contract in code + the local-first default.
//
// Pins (the RED-before form over the language surface, both lanes —
// TW (`metalogos::run_program`) and the bytecode VM):
//   T1: LOCAL-FIRST DEFAULT — a program with NO `llm {}` block and NO
//       external declaration (no provider/key/base-url env) runs on the
//       LOCAL provider: the deterministic, non-echo, offline
//       `[mock-llm:<hex>]` marker. The №454 security essence holds: the
//       prompt NEVER reaches the caller, and the local answer is
//       VISIBLE as such (ADR-0208 §2.2 — the ONE intentional posture
//       change; the old default was the loud real backend).
//   T2: the DECLARED EXTERNAL lane keeps the №454 fail-loud posture
//       exactly — `METALOGOS_LLM_PROVIDER` set without a key refuses at
//       the first call, naming `METALOGOS_API_KEY`.
//   T3: the EMPTY `llm { }` block is the local-first default at the
//       router's empty-chain seam — the block that declares nothing
//       external runs on the local provider (ADR-0208 §2.2).
//   T4: the `call_llm` builtin's fallback seam composes the same way
//       (the local-first default → the marker; the explicit mock
//       opt-in keeps its exact `[MOCK: …]` declared semantics — №478).
//   T5: the contract surface itself — the deterministic report
//       (byte-identical double-run) and the closed-roster lookup (an
//       unaddressable name refuses loudly through the EXISTING
//       `LLM_PROVIDER_UNAVAILABLE` class; №385 vocabulary frozen).
//
// Env discipline (the №478 form): every env-mutating case holds the
// process-global ENV_LOCK; №401 hygiene — no test ever leaves a key or
// a provider env behind, and no real network is touched (the local
// provider is offline; the declared-external lane refuses at the key
// check, before any request).

use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// №401/№478 hygiene: clear EVERY declaration surface — the local-first
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

/// The learnable-pattern fixture (the create_llm_backend seam — the
/// same channel the №454 serve repro drives). The marker lives ONLY in
/// the prompt; any echo of the prompt leaks it.
const N879_LEARNABLE: &str = r#"
learnable pattern Tag(text: String) -> String {
  prompt: "classify the caller text"
  context: "session transcript: N879_SECRET_MARKER_ALPHA"
}
flow Main { input: String = "tell-me" -> Tag -> output }
"#;

/// Compile-and-run the fixture on the bytecode VM (the №454 harness
/// form): parse → compile → fresh Vm → run.
fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let base_dir = std::env::current_dir().map_err(|e| format!("cwd error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir);
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_no_marker(text: &str, what: &str) {
    assert!(
        !text.contains("N879_SECRET_MARKER_ALPHA"),
        "{}: the prompt marker must NEVER reach the caller (the №454 essence) — got: {}",
        what,
        text
    );
}

fn assert_visible_local(answer: &str, what: &str) {
    assert!(
        answer.starts_with("[mock-llm:") && answer.ends_with("]"),
        "{}: the local-first answer must be the VISIBLE deterministic marker (ADR-0208 §2.2), got: {}",
        what,
        answer
    );
    assert_no_marker(answer, what);
}

// ── T1: the local-first default, both lanes ──────────────────────────

#[test]
fn n879_t1_local_first_default_tw_lane() {
    let _env = lock_env();
    clear_llm_env();
    let out = metalogos::run_program(N879_LEARNABLE)
        .expect("the local-first default must answer (ADR-0208 §2.2)")
        .unwrap_or_default();
    assert_visible_local(&out, "TW lane local-first");
}

#[test]
fn n879_t1_local_first_default_vm_lane() {
    let _env = lock_env();
    clear_llm_env();
    let out = run_vm(N879_LEARNABLE)
        .expect("the VM lane answers on the local-first default too (the shared seam)")
        .unwrap_or_default();
    assert_visible_local(&out, "VM lane local-first");
}

// ── T2: the declared external lane keeps the №454 fail-loud posture ──

#[test]
fn n879_t2_declared_external_fails_loud_tw_lane() {
    let _env = lock_env();
    clear_llm_env();
    std::env::set_var("METALOGOS_LLM_PROVIDER", "anthropic");
    let result = metalogos::run_program(N879_LEARNABLE);
    std::env::remove_var("METALOGOS_LLM_PROVIDER");
    let err = result.expect_err(
        "a DECLARED external provider without credentials must refuse loudly (№454 preserved)",
    );
    assert_no_marker(&err, "TW declared-external");
    assert!(
        err.contains("METALOGOS_API_KEY"),
        "the loud refusal must name the missing key — got: {}",
        err
    );
}

#[test]
fn n879_t2_declared_external_fails_loud_vm_lane() {
    let _env = lock_env();
    clear_llm_env();
    std::env::set_var("METALOGOS_LLM_PROVIDER", "anthropic");
    let result = run_vm(N879_LEARNABLE);
    std::env::remove_var("METALOGOS_LLM_PROVIDER");
    let err = result.expect_err("the VM lane refuses loudly on the declared-external lane too");
    assert_no_marker(&err, "VM declared-external");
    assert!(
        err.contains("METALOGOS_API_KEY"),
        "the VM parity refusal names the missing key — got: {}",
        err
    );
}

// ── T3: the empty llm{} block — the router's empty-chain seam ────────

#[test]
fn n879_t3_empty_llm_block_runs_local_first() {
    let _env = lock_env();
    clear_llm_env();
    // A clean install: clear any router a previous test left behind —
    // the empty block under test installs its own.
    metalogos::llm::clear_global_smart_router();
    // An `llm { }` block with NO provider entries: the block declares
    // nothing external — the empty-chain seam composes the local
    // provider (ADR-0208 §2.2; the SmartRouter empty-providers branch
    // rides create_llm_backend).
    let src = format!("llm {{ }}\n{}", N879_LEARNABLE);
    let out = metalogos::run_program(&src)
        .expect("the empty llm{} block is the local-first default, never an error")
        .unwrap_or_default();
    assert_visible_local(&out, "empty llm{} block");
}

// ── T4: the call_llm builtin's fallback seam ─────────────────────────

#[test]
fn n879_t4_call_llm_fallback_seam_composes_the_same_way() {
    let _env = lock_env();
    clear_llm_env();
    // The process-global router hygiene: a previous test's llm{} install
    // (T3's empty block persists in GLOBAL_SMART_ROUTER) would route this
    // call through the empty-chain seam — the builtin's own fallback lane
    // (the [MOCK: branch of №478) is reachable only WITHOUT a router.
    // Clear it: the seam under test is the fallback composition.
    metalogos::llm::clear_global_smart_router();
    let src = r#"
pattern Ask(question: String) -> String {
  return call_llm("Answer:", question)
}
flow Main { input: String = "hi" -> Ask -> output }
"#;
    // Nothing declared → the fallback seam composes the LOCAL provider:
    // the visible marker, never the old loud real-backend failure and
    // never the explicit-mock echo lane.
    let out = metalogos::run_program(src)
        .expect("call_llm with nothing declared answers on the local provider")
        .unwrap_or_default();
    assert_visible_local(&out, "call_llm fallback seam");
    assert!(
        !out.contains("[MOCK:"),
        "the [MOCK:] lane is the EXPLICIT opt-in only (№478 declared semantics) — got: {}",
        out
    );

    // The explicit mock opt-in keeps its exact declared semantics (1:1,
    // ADR-0208 §6): the `[MOCK: prompt | input]` answer of the builtin's
    // own mock branch.
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    let out = metalogos::run_program(src).expect("the explicit mock opt-in keeps answering (№478)");
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let out = out.unwrap_or_default();
    assert!(
        out.contains("[MOCK:"),
        "the explicit opt-in must keep its exact [MOCK:] declared semantics — got: {}",
        out
    );
}

// ── T5: the contract surface itself ──────────────────────────────────

#[test]
fn n879_t5_contract_report_double_run_byte_identical() {
    use metalogos::provider_contract::{contract_for_name, ProviderContract};
    let views = [
        ProviderContract::local(),
        contract_for_name("anthropic").expect("roster entry"),
        contract_for_name("openai").expect("roster entry"),
        contract_for_name("ollama").expect("roster entry"),
    ];
    for contract in &views {
        assert_eq!(
            contract.report().as_bytes(),
            contract.report().as_bytes(),
            "the contract report is byte-identical across runs (№879)"
        );
        // The canonical shape: four deterministic lines, no timestamps.
        assert_eq!(contract.report().lines().count(), 4);
    }
}

#[test]
fn n879_t5_unaddressable_provider_refuses_loudly_existing_class() {
    use metalogos::provider_contract::contract_for_name;
    let err =
        contract_for_name("not-a-provider").expect_err("a name outside the closed roster refuses");
    assert!(
        err.starts_with("[LLM_PROVIDER_UNAVAILABLE]"),
        "the refusal rides the EXISTING class at the front (№385 frozen vocabulary) — got: {}",
        err
    );
    assert!(
        err.contains("not-a-provider"),
        "the refusal names the forged name — got: {}",
        err
    );
}
