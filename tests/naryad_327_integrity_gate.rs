//! Наряд №327 (issue #421) — the integrity axis and anti-injection:
//! data that DECIDES control flow must be `trusted`; untrusted data as
//! DATA is legal.
//!
//! Covers:
//! - (а) red/green: the LLM answer driving a branch around a destructive
//!   action does NOT compile (UNTRUSTED_DECISION names the source and
//!   the decision point); the same answer as DATA compiles;
//! - (б) all three decision positions: if (incl. else-if), while,
//!   match scrutinee;
//! - (в) the integrity join: untrusted poisons derivatives (untrusted
//!   + trusted = untrusted) — the gate sees through wrappers;
//! - (г) trusted/private decisions are legal (conf is not integrity);
//! - (д) zero delta for plain programs; the showcase example;
//! - (е) no-stub hygiene.
//!
//! Boundary (loud, per the naryad): content-level injection analysis is
//! out of the compiler's scope; media sources are Phase 2; taint
//! polymorphism deferred.

use metalogos::parser::parse;
use metalogos::semantic::check_program;

fn compile_errs(source: &str) -> Vec<String> {
    match metalogos::compile_program(source) {
        Ok(_) => Vec::new(),
        Err(e) => e.lines().map(str::to_string).collect(),
    }
}

// ── (а) The naryad scenario: decision vs data ────────────────────────

#[test]
fn n327_llm_answer_deciding_a_branch_does_not_compile() {
    let src = r#"
        pattern Decide() -> String {
            let answer = call_llm("shutdown the service?")
            if answer == "yes" {
                let _ = exec("shutdown -h now")
                return "shutting down"
            }
            return "kept running"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter().any(|m| m.contains("[UNTRUSTED_DECISION]")
            && m.contains("source 'call_llm'")
            && m.contains("decides a 'if'")),
        "the LLM answer must not decide the branch: {errs:?}"
    );
}

#[test]
fn n327_llm_answer_as_data_compiles() {
    let src = r#"
        pattern Carry(data: String) -> String {
            let answer = call_llm("summarize")
            let shaped = upper(answer)
            return shaped + "|" + data
        }
    "#;
    assert!(
        compile_errs(src).is_empty(),
        "untrusted data as DATA is legal — the gate fires on decisions only"
    );
}

#[test]
fn n327_the_showcase_example_pins_both_sides() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{manifest_dir}/examples/l1_injection.mlog"))
        .expect("example");
    let errs = compile_errs(&src);
    assert!(
        errs.iter().any(|m| m.contains("[UNTRUSTED_DECISION]")
            && m.contains("pattern Decide")
            && m.contains("source 'call_llm'")),
        "Decide must be rejected with the source named: {errs:?}"
    );
    // The data side (Carry) is NOT reported.
    assert!(
        !errs.iter().any(|m| m.contains("pattern Carry")),
        "Carry is the legal data path: {errs:?}"
    );
}

// ── (б) All three decision positions ─────────────────────────────────

#[test]
fn n327_while_condition_is_a_decision() {
    let src = r#"
        pattern Loop() -> String {
            let cmd = http_get("https://attacker.example/flag")
            let mut done = "no"
            while cmd != "stop" {
                done = "looping"
            }
            return done
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter().any(|m| m.contains("[UNTRUSTED_DECISION]")
            && m.contains("decides a 'while'")
            && m.contains("source 'http_get'")),
        "while on untrusted data is a decision: {errs:?}"
    );
}

#[test]
fn n327_match_scrutinee_is_a_decision() {
    let src = r#"
        pattern Route() -> String {
            let answer = call_llm("pick: a or b")
            let mut out = "x"
            match answer {
                "a" then { out = "path-a" }
                else { out = "path-b" }
            }
            return out
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter()
            .any(|m| m.contains("[UNTRUSTED_DECISION]") && m.contains("decides a 'match'")),
        "match on untrusted data is a decision: {errs:?}"
    );
}

#[test]
fn n327_else_if_conditions_are_decisions() {
    let src = r#"
        pattern Chain(flag: String) -> String {
            let answer = call_llm("yes or no")
            if flag == "1" {
                return "one"
            } else if answer == "yes" {
                return "two"
            }
            return "three"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter()
            .any(|m| m.contains("[UNTRUSTED_DECISION]") && m.contains("source 'call_llm'")),
        "else-if conditions are decisions: {errs:?}"
    );
}

// ── (в) The integrity join poisons derivatives ───────────────────────

#[test]
fn n327_untrusted_poisons_derivatives_through_join() {
    // untrusted + trusted = untrusted: wrapping the answer in pure
    // compute does not launder the integrity — the gate still fires.
    let src = r#"
        pattern Decide() -> String {
            let answer = call_llm("yes or no")
            let wrapped = upper(trim(answer + "!"))
            if wrapped == "YES!" {
                return "branch"
            }
            return "other"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter().any(|m| m.contains("[UNTRUSTED_DECISION]")),
        "join integrity: derived data keeps untrusted: {errs:?}"
    );
}

// ── (г) Legal decisions ──────────────────────────────────────────────

#[test]
fn n327_private_but_trusted_decisions_are_legal() {
    // conf is NOT integrity: a private-but-trusted value may decide.
    let src = r#"
        pattern Decide(flag: String<private>) -> String {
            if flag == "go" {
                return "went"
            }
            return "stayed"
        }
    "#;
    assert!(
        compile_errs(src).is_empty(),
        "private+trusted deciding is legal (conf ≠ integrity)"
    );
}

#[test]
fn n327_one_way_redact_restores_trust_for_decisions() {
    // The sanctioned path: hash_only destroys the data AND lifts
    // integrity (the label becomes trusted) — the decision is legal.
    let src = r#"
        pattern Decide() -> String {
            let answer = redact(call_llm("yes or no"), "hash_only")
            if answer == "x" {
                return "branch"
            }
            return "other"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        !errs.iter().any(|m| m.contains("[UNTRUSTED_DECISION]")),
        "a one-way policy restores integrity: {errs:?}"
    );
}

// ── (д) Zero delta ───────────────────────────────────────────────────

#[test]
fn n327_plain_programs_stay_clean() {
    let src = r#"
        entity threshold: Float = 0.9
        pattern Gate(x: Float) -> String {
            if x > threshold {
                return "high"
            }
            return "low"
        }
    "#;
    assert!(
        compile_errs(src).is_empty(),
        "literal/param decisions are trusted — zero delta"
    );
    let decls = parse(
        r#"
        pattern P(x: String) -> String {
            if x == "a" { return "1" }
            return "2"
        }
        "#,
    )
    .unwrap();
    let result = check_program(&decls);
    assert!(result.is_ok(), "{:?}", result.format());
}

// ── (е) No-stub hygiene (№16.0-D) ────────────────────────────────────

#[test]
fn n327_no_stub_markers_in_touched_files() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in ["src/semantic.rs", "src/audit.rs"] {
        let src = std::fs::read_to_string(format!("{manifest_dir}/{file}"))
            .unwrap_or_else(|_| panic!("{file} must exist"));
        for marker in ["todo!", "unimplemented!", "SKELETON"] {
            assert!(
                !src.contains(marker),
                "stub marker `{marker}` found in {file}"
            );
        }
    }
}
