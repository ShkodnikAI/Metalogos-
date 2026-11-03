// ── tests/naryad_880_failover_coverage.rs ────────────────────────────
// Наряд №880 (issue #1740, wave 78 — W-G, P1, [llm][core], ADR-0208 §4):
// the coverage rule — the fail-closed gate on provider dependence.
//
// The rule: an LLM call addressed to an external provider must END in a
// covered local fallback path (a `local` rung in the failover roster,
// grammar.pest:39) or in the HONEST refusal (the existing exhaustion
// class — LLM_PROVIDER_UNAVAILABLE / the loud "All LLM providers
// failed", the frozen №385 vocabulary: no new deny codes); never in a
// silent degradation. The pre-network refusal half: a rung whose
// provider class REQUIRES an API key (anthropic) refuses BEFORE any
// HTTP wait.
//
// The e2e trace (the naryad's §2): request → the deterministic external
// refusal (the unreachable loopback stub — connect refused, no external
// network) → the failover chain → the local fallback → the result. The
// double run is byte-identical. Backend-agnostic: the tree-walking
// interpreter and the VM lane give ONE outcome (the №825/№829 two-lane
// form — both install the same SmartRouter).
//
// The №874 form (the fail-closed I/O gate of wave 77) is the precedent;
// the egress perimeter itself is NOT duplicated — this rule is about
// the LLM fallback PATHS, not the network boundary.

use std::sync::Mutex;

/// Serialize the whole file — the tests mutate the process-global LLM
/// state (the router installs on run_program, the №251 discipline).
static FILE_LOCK: Mutex<()> = Mutex::new(());

fn lock_file() -> std::sync::MutexGuard<'static, ()> {
    FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// №401 hygiene: no test ever dials a real host — the dead rung is the
/// loopback port 1 (connect refused, deterministic, instant).
const DEAD_URL: &str = "http://127.0.0.1:1/v1/chat/completions";

/// Serialize a source's llm {} declaration through from_config.
fn router_for(source: &str) -> metalogos::llm::SmartRouter {
    let decls = metalogos::parser::parse(source).expect("llm decl must parse");
    let config = decls
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::LlmConfig(c) => Some(c.clone()),
            _ => None,
        })
        .expect("llm decl present");
    metalogos::llm::SmartRouter::from_config(&config)
}

/// Compile-and-run on the bytecode VM (the №825 harness form).
fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let base_dir = std::env::current_dir().map_err(|e| format!("cwd error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir);
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_visible_local(answer: &str, what: &str) {
    assert!(
        answer.starts_with("[mock-llm:") && answer.ends_with("]"),
        "{}: the local fallback answers the visible deterministic marker, got: {}",
        what,
        answer
    );
}

// ── The rule's machine probe ────────────────────────────────────────

#[test]
fn n880_covered_roster_probe() {
    let _g = lock_file();
    let covered = router_for(&format!(
        "llm {{ providers: [{{alias: dead, provider: vendorx, url: \"{DEAD_URL}\"}}, {{alias: l, provider: local}}] }}"
    ));
    assert!(
        covered.has_covered_local_fallback(),
        "the roster with a local rung IS covered"
    );

    let uncovered = router_for(&format!(
        "llm {{ providers: [{{alias: dead, provider: vendorx, url: \"{DEAD_URL}\"}}] }}"
    ));
    assert!(
        !uncovered.has_covered_local_fallback(),
        "the external-only roster is NOT covered — the honest-refusal half governs"
    );

    // №879's composition: the EMPTY roster is covered by construction
    // (the empty-chain seam composes the local provider when nothing
    // external is declared — the local-first default IS coverage).
    let local_only = router_for("llm { }");
    assert!(
        local_only.has_covered_local_fallback(),
        "the local-first default IS the covered roster"
    );
}

// ── The e2e trace: external refusal → failover → local fallback ─────

#[test]
fn n880_e2e_failover_to_local_rung_router_level() {
    let _g = lock_file();
    let router = router_for(&format!(
        "llm {{ failover: auto, providers: [{{alias: dead, provider: vendorx, url: \"{DEAD_URL}\"}}, {{alias: l, provider: local}}] }}"
    ));

    let r1 = router
        .call("Say something", "world", None, None)
        .expect("the covered roster must answer via the local rung");
    assert_visible_local(&r1, "router-level e2e");

    // The double run — byte-identical (the №874 determinism form).
    let r2 = router
        .call("Say something", "world", None, None)
        .expect("the second run answers too");
    assert_eq!(r1, r2, "the e2e trace is byte-identical across runs");
}

/// The program-level e2e on BOTH lanes — the naryad's backend-agnostic
/// gate: the TW (tree-walking) interpreter and the VM give ONE outcome.
fn covered_program() -> String {
    format!(
        r#"
llm {{
  failover: auto,
  providers: [
    {{alias: dead, provider: vendorx, url: "{DEAD_URL}"}},
    {{alias: fallback, provider: local}}
  ]
}}
pattern Ask(question: String) -> String {{
  return call_llm("Answer:", question)
}}
flow Main {{ input: String = "hi" -> Ask -> output }}
"#
    )
}

#[test]
fn n880_e2e_failover_to_local_rung_tw_lane() {
    let _g = lock_file();
    let src = covered_program();
    let out1 = metalogos::run_program(&src)
        .expect("the covered roster answers on the TW lane")
        .unwrap_or_default();
    assert_visible_local(&out1, "TW lane e2e");
    // The double run — byte-identical.
    let out2 = metalogos::run_program(&src)
        .expect("the second TW run answers too")
        .unwrap_or_default();
    assert_eq!(out1, out2, "the TW e2e trace is byte-identical across runs");
}

#[test]
fn n880_e2e_failover_to_local_rung_vm_lane() {
    let _g = lock_file();
    let src = covered_program();
    let out1 = run_vm(&src)
        .expect("the covered roster answers on the VM lane (the same router)")
        .unwrap_or_default();
    assert_visible_local(&out1, "VM lane e2e");
    let out2 = run_vm(&src)
        .expect("the second VM run answers too")
        .unwrap_or_default();
    assert_eq!(out1, out2, "the VM e2e trace is byte-identical across runs");
}

// ── The honest refusal (the uncovered half) ─────────────────────────

#[test]
fn n880_uncovered_roster_refuses_honest_no_new_codes() {
    let _g = lock_file();
    let router = router_for(&format!(
        "llm {{ providers: [{{alias: d1, provider: vendorx, url: \"{DEAD_URL}\"}}] }}"
    ));
    let err = router
        .call("Say something", "world", None, None)
        .expect_err("the uncovered dead roster must refuse honestly");
    assert!(
        err.contains("All LLM providers failed"),
        "the existing loud class expected, got: {}",
        err
    );
    assert!(
        !err.contains("[LLM_") || err.starts_with("[LLM_"),
        "no forged mid-text class stamps (№385 split rule), got: {}",
        err
    );
}

// ── The pre-network refusal half ────────────────────────────────────

#[test]
fn n880_keyless_refusal_happens_before_the_network() {
    let _g = lock_file();
    // The key-bearing requirement is checked BEFORE any HTTP attempt —
    // the «отказ до сети» half of the rule. The refusal rides the
    // EXISTING class and is visible in the standard error output.
    let router = router_for("llm { providers: [{alias: a, provider: anthropic}] }");
    let t0 = std::time::Instant::now();
    let err = router
        .call("Say something", "world", None, None)
        .expect_err("a keyless anthropic rung must refuse");
    let elapsed = t0.elapsed();
    assert!(
        err.contains("anthropic requires an API key") && err.contains("All LLM providers failed"),
        "the existing refusal class expected (the keyless pre-network refusal surfaced \
         through the exhaustion wrapper), got: {}",
        err
    );
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "the refusal happened before any network wait, took {:?}",
        elapsed
    );
}

// ── The contract view follows the local rung (the №880 refinement) ──

#[test]
fn n880_contract_view_all_local_roster_is_deterministic() {
    let _g = lock_file();
    use metalogos::provider_contract::ProviderContract;
    // An ALL-LOCAL roster is deterministic — every rung answers the
    // frozen marker; ANY external rung breaks the guarantee.
    let decls = metalogos::parser::parse(
        "llm { providers: [{alias: a, provider: local}, {alias: b, provider: local}] }",
    )
    .expect("the all-local roster must parse");
    let config = match &decls[0] {
        metalogos::ast::Declaration::LlmConfig(c) => c.clone(),
        _ => panic!("expected LlmConfig"),
    };
    let view = ProviderContract::from_config(&config);
    assert!(
        view.capabilities.deterministic_answer,
        "the all-local roster IS deterministic"
    );
    // …and the covered roster with an external rung is not.
    let decls = metalogos::parser::parse(&format!(
        "llm {{ providers: [{{alias: d, provider: vendorx, url: \"{DEAD_URL}\"}}, {{alias: l, provider: local}}] }}"
    ))
    .expect("the covered roster must parse");
    let config = match &decls[0] {
        metalogos::ast::Declaration::LlmConfig(c) => c.clone(),
        _ => panic!("expected LlmConfig"),
    };
    let view = ProviderContract::from_config(&config);
    assert!(
        !view.capabilities.deterministic_answer,
        "the mixed roster is NOT deterministic (the external rung answers through the network)"
    );
}
