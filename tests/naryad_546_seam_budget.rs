// ── tests/naryad_546_seam_budget.rs — №546 (issue #884) ─────────────
//
// ADR-0178 §5 preconditions 4–5, end-to-end over both backends:
//
//   4. the runtime SECRET check at the embedding seam — a value of the
//      secret family (`secret()` builtin) never enters `embed()` /
//      `semantic_search()`; the refusal is LOUD and seam-named
//      (`[EMBED_SECRET_REJECTED]`), on TW and VM alike;
//   5. the CONTOUR CALL BUDGET — embedding operations are limited per
//      scope (`METALOGOS_CONTOUR_BUDGET`, default 64); the refusal is
//      loud (`[CONTOUR_BUDGET_EXCEEDED]`), fail-closed, no silent
//      degradation.
//
// Run: cargo test --features vec --test naryad_546_seam_budget
// The file is empty without the `vec` feature (the №272 convention:
// the no-feature build stays green).
//
// Serial: the budget counter is thread-local and the env var is
// process-global; the tests construct their own scope (reset-on-entry)
// and restore the env explicitly.

#![cfg(feature = "vec")]

use serial_test::serial;

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, std::path::PathBuf::from("."))
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(std::path::Path::new(".").to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// A scope resets the thread-local budget counter — the test-side
/// equivalent of the request/tick boundary guard.
fn fresh_scope() {
    let _ = metalogos::builtins::ContourBudgetScope::new();
}

struct EnvGuard(&'static str);
impl EnvGuard {
    fn set(var: &'static str, val: &str) -> Self {
        std::env::set_var(var, val);
        EnvGuard(var)
    }
}
impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var(self.0);
    }
}

#[test]
#[serial]
fn naryad_546_embed_refuses_the_secret_family_loudly_on_tw_and_vm() {
    let _env = EnvGuard::set("METALOGOS_546_TEST_SECRET", "hunter2-value");
    fresh_scope();

    // The secret family must never reach the embedding model. The
    // refusal carries the seam's name — a generic type error would not
    // pin the guarantee.
    let src = r#"
pattern Probe(_input: String) -> String {
    let s = secret("METALOGOS_546_TEST_SECRET")
    return embed(s)
}
pattern RunAll(_input: String) -> String {
    return Probe("")
}

flow Main { input: String = "x" -> RunAll -> output }
"#;

    let err_tw = run_tw(src).expect_err("TW: embed(secret) must fail loudly");
    assert!(
        err_tw.contains("[EMBED_SECRET_REJECTED]"),
        "TW: the refusal must be seam-named, got: {err_tw}"
    );
    assert!(
        err_tw.contains("redact()"),
        "TW: the error must name the legal masking path, got: {err_tw}"
    );

    let err_vm = run_vm(src).expect_err("VM: embed(secret) must fail loudly");
    assert!(
        err_vm.contains("[EMBED_SECRET_REJECTED]"),
        "VM: the refusal must be seam-named, got: {err_vm}"
    );

    // The masked path stays legal: redact() is the ONE sanctioned way a
    // secret-derived text reaches the seam.
    let masked_src = r#"
pattern Probe(_input: String) -> String {
    let s = secret("METALOGOS_546_TEST_SECRET")
    let masked = redact(s, "all")
    let v = embed(masked)
    return str(len(v) > 0)
}
pattern RunAll(_input: String) -> String {
    return Probe("")
}

flow Main { input: String = "x" -> RunAll -> output }
"#;
    let _ = run_tw(masked_src).expect("TW: embed(redact(secret)) is the legal path");
    let _ = run_vm(masked_src).expect("VM: embed(redact(secret)) is the legal path");
}

#[test]
#[serial]
fn naryad_546_semantic_search_refuses_the_secret_query_loudly() {
    let _env = EnvGuard::set("METALOGOS_546_TEST_SECRET", "hunter2-value");
    fresh_scope();

    let src = r#"
pattern Probe(_input: String) -> String {
    let s = secret("METALOGOS_546_TEST_SECRET")
    let hits = semantic_search(s, ["doc one", "doc two"], "2")
    return str(len(hits))
}
pattern RunAll(_input: String) -> String {
    return Probe("")
}

flow Main { input: String = "x" -> RunAll -> output }
"#;
    for (backend, res) in [("TW", run_tw(src)), ("VM", run_vm(src))] {
        let err = res.expect_err(
            format!("{backend}: semantic_search(secret, ...) must fail loudly").as_str(),
        );
        assert!(
            err.contains("[EMBED_SECRET_REJECTED]"),
            "{backend}: seam-named refusal expected, got: {err}"
        );
    }
}

#[test]
#[serial]
fn naryad_546_budget_refuses_over_the_limit_loudly_on_tw_and_vm() {
    let _env = EnvGuard::set("METALOGOS_546_TEST_SECRET", "hunter2-value");
    let _budget = EnvGuard::set("METALOGOS_CONTOUR_BUDGET", "3");

    // 4 embed calls against a 3-unit budget: the 4th refuses loudly.
    let src = r#"
pattern Probe(_input: String) -> String {
    let a = embed("первый документ про котов и ковры")
    let b = embed("второй документ про телескопы и орбиты")
    let c = embed("третий документ про рецепты и кухни")
    let d = embed("четвёртый документ превышает бюджет шва")
    return str(a[0]) + str(b[0]) + str(c[0]) + str(d[0])
}
pattern RunAll(_input: String) -> String {
    return Probe("")
}

flow Main { input: String = "x" -> RunAll -> output }
"#;
    for (backend, res) in [("TW", run_tw(src)), ("VM", run_vm(src))] {
        let err = res.expect_err(format!("{backend}: the 4th embed must hit the budget").as_str());
        assert!(
            err.contains("[CONTOUR_BUDGET_EXCEEDED]"),
            "{backend}: loud budget refusal expected, got: {err}"
        );
    }
}

#[test]
#[serial]
fn naryad_546_budget_within_the_limit_stays_green_and_the_scope_resets() {
    let _env = EnvGuard::set("METALOGOS_546_TEST_SECRET", "hunter2-value");
    let _budget = EnvGuard::set("METALOGOS_CONTOUR_BUDGET", "3");

    let src = r#"
pattern Probe(_input: String) -> String {
    let a = embed("первый документ про котов и ковры")
    let b = embed("второй документ про телескопы и орбиты")
    let c = embed("третий документ про рецепты и кухни")
    return str(a[0] >= 0.0) + str(b[0] >= 0.0) + str(c[0] >= 0.0)
}
pattern RunAll(_input: String) -> String {
    return Probe("")
}

flow Main { input: String = "x" -> RunAll -> output }
"#;
    fresh_scope();
    let _ = run_tw(src).expect("TW: 3 embeds against a 3-unit budget fit");
    // A fresh scope resets the counter (the request/tick boundary) — the
    // same program fits again without touching the env.
    fresh_scope();
    let _ = run_vm(src).expect("VM: after a new scope the same 3 embeds fit");
}
