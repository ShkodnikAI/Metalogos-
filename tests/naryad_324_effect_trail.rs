//! Наряд №324 (issue #418) — effect trail in pattern signatures
//! (ADR-0154 §9): syntax `⟨io, audit⟩`, the boundary gate (factual
//! effects of the body ⊑ declared trail), the №316 SSOT mapping for
//! builtin effects, the recursion rule, and the zero-delta property.
//!
//! Boundary (loud, per the naryad): grant effects are Phase 3 (№339);
//! polymorphic effects deferred; the runtime parity of trails is №328.

use metalogos::ast::{Declaration, Effect, EffectSet};
use metalogos::parser::parse;
use metalogos::semantic::check_program;

fn parse_ok(source: &str) -> Vec<Declaration> {
    parse(source).unwrap_or_else(|e| panic!("parse failed: {e}"))
}

fn check_errors(source: &str) -> Vec<String> {
    check_program(&parse_ok(source))
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

fn effects_of(words: &[&str]) -> EffectSet {
    let mut s = EffectSet::new();
    for w in words {
        s.insert(Effect::parse_word(w).unwrap_or_else(|| panic!("bad word {w}")));
    }
    s
}

// ── (а) Syntax: the trail parses, carries a span, type stays bare ────

#[test]
fn n324_effect_trail_parses_and_carries_span() {
    let decls = parse_ok(
        r#"
        pattern P(x: String) -> String ⟨io, audit⟩ { return x }
        "#,
    );
    let Declaration::Pattern(p) = &decls[0] else {
        panic!("pattern");
    };
    let ann = p.effects.as_ref().expect("trail must be carried");
    assert_eq!(ann.raw, "io, audit");
    assert!(ann.span.start_line > 0, "trail must carry a span");
    assert_eq!(p.return_type, "String", "type name stays bare");
}

#[test]
fn n324_trail_parses_on_tool_method_and_learnable() {
    let decls = parse_ok(
        r#"
        tool tg { send(m: String) -> String ⟨io⟩ { return m } }
        learnable pattern Classify(x: String) -> String ⟨io⟩ { prompt: "classify" }
        "#,
    );
    let mut saw_tool = false;
    let mut saw_learnable = false;
    for d in &decls {
        match d {
            Declaration::Tool(t) => {
                let m = &t.methods[0];
                assert_eq!(m.effects.as_ref().expect("trail").raw, "io");
                saw_tool = true;
            }
            Declaration::LearnablePattern(lp) => {
                assert_eq!(lp.effects.as_ref().expect("trail").raw, "io");
                saw_learnable = true;
            }
            _ => {}
        }
    }
    assert!(saw_tool && saw_learnable);
}

#[test]
fn n324_empty_trail_is_the_zero_effect_declaration() {
    let decls = parse_ok(
        r#"
        pattern Pure() -> String ⟨⟩ { return "x" }
        "#,
    );
    let Declaration::Pattern(p) = &decls[0] else {
        panic!("pattern");
    };
    let ann = p.effects.as_ref().expect("trail carried");
    assert_eq!(ann.raw, "");
    // ⟨⟩ with a body that does nothing: valid, no errors.
    let errs: Vec<String> = check_program(&decls)
        .errors
        .iter()
        .filter(|e| e.message.contains("effect"))
        .map(|e| e.message.clone())
        .collect();
    assert!(errs.is_empty(), "clean pure pattern must pass: {errs:?}");
}

#[test]
fn n324_unknown_effect_word_is_loud() {
    let errs = check_errors(
        r#"
        pattern P() -> String ⟨network⟩ { return "x" }
        "#,
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("unknown effect word 'network'")),
        "unknown trail word must be a loud error: {errs:?}"
    );
}

#[test]
fn n324_duplicate_effect_word_is_loud() {
    let errs = check_errors(
        r#"
        pattern P() -> String ⟨io, io⟩ { return "x" }
        "#,
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("duplicate effect word 'io'")),
        "duplicate trail word must be loud: {errs:?}"
    );
}

// ── (б) The boundary gate: factual ⊑ declared ────────────────────────

#[test]
fn n324_exceeding_call_is_a_compile_error_with_the_list() {
    // Log's body memorizes (audit) but declares only ⟨io⟩.
    let errs = check_errors(
        r#"
        pattern Log(msg: String) -> String ⟨io⟩ {
            memorize msg with priority=0.5
            return msg
        }
        "#,
    );
    assert!(
        errs.iter().any(|m| {
            m.contains("effect trail violation")
                && m.contains("declared ⟨io⟩")
                && m.contains("body requires ⟨io, audit⟩")
                && m.contains("excess: audit")
        }),
        "exceeding trail must be a loud error with the effect list: {errs:?}"
    );
}

#[test]
fn n324_exceeding_call_through_a_helper_is_a_compile_error() {
    // Fetch declares ⟨io⟩ (env → io: fine). Log declares ⟨io⟩ but its
    // body calls MemorizePattern whose DECLARED contract is ⟨io, audit⟩:
    // the violation is caught on MemorizePattern itself.
    let errs = check_errors(
        r#"
        pattern Save(v: String) -> String ⟨io, audit⟩ {
            memorize v with priority=0.5
            return v
        }
        pattern Log(msg: String) -> String ⟨io⟩ {
            return Save(msg)
        }
        "#,
    );
    // Interface semantics: Save's DECLARED trail ⟨io, audit⟩ is the call
    // contract — so Log, which declared only ⟨io⟩ but CALLS a contract
    // requiring audit, is the violating boundary.
    assert!(
        errs.iter()
            .any(|m| m.contains("pattern 'Log'") && m.contains("excess: audit")),
        "the violation is caught at the calling boundary: {errs:?}"
    );
    // And Save itself is clean: its body matches its own declared trail.
    assert!(
        !errs.iter().any(|m| m.contains("pattern 'Save'")),
        "a body matching its own declared trail stays clean: {errs:?}"
    );
}

#[test]
fn n324_undeclared_callee_shares_its_factual_effects() {
    // No annotations anywhere: the caller's DECLARED trail ⟨⟩ (pure)
    // must fail, because the callee's factual effects ({io} from env)
    // ride the call.
    let errs = check_errors(
        r#"
        pattern Fetch() -> String {
            return env("K")
        }
        pattern Run() -> String ⟨⟩ {
            return Fetch()
        }
        "#,
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("pattern 'Run'") && m.contains("excess: io")),
        "an undeclared callee's factual effects ride the call: {errs:?}"
    );
}

#[test]
fn n324_legal_call_compiles_clean() {
    let decls = parse_ok(
        r#"
        pattern Fetch() -> String ⟨io⟩ {
            return env("K")
        }
        pattern Log(v: String) -> String ⟨io, audit⟩ {
            memorize v with priority=0.5
            return v
        }
        pattern Run() -> String ⟨io, audit⟩ {
            let k = Fetch()
            return Log(k)
        }
        "#,
    );
    let errs: Vec<String> = check_program(&decls)
        .errors
        .iter()
        .filter(|e| e.message.contains("effect"))
        .map(|e| e.message.clone())
        .collect();
    assert!(errs.is_empty(), "legal composition must pass: {errs:?}");
}

#[test]
fn n324_no_trail_means_no_gate_zero_delta() {
    // Existing programs: no trail anywhere — the gate does not apply,
    // even when bodies do io/audit.
    let decls = parse_ok(
        r#"
        pattern LogIt(msg: String) -> String {
            memorize msg with priority=0.5
            return msg
        }
        pattern FetchIt() -> String {
            return env("K")
        }
        "#,
    );
    let result = check_program(&decls);
    assert!(
        result.is_ok(),
        "unannotated patterns must stay clean: {:?}",
        result.format()
    );
}

// ── (в) Builtin effects via the №316 SSOT ────────────────────────────

#[test]
fn n324_builtin_source_is_io_sink_with_side_effect_is_io_and_audit() {
    // env: Source → {io}. print: Sink/Irreversible → {io, audit}.
    let errs = check_errors(
        r#"
        pattern Reader() -> String ⟨⟩ {
            return env("K")
        }
        "#,
    );
    assert!(
        errs.iter().any(|m| m.contains("excess: io")),
        "env is a Source → io: {errs:?}"
    );

    let errs = check_errors(
        r#"
        pattern Printer() -> String ⟨⟩ {
            print("hello")
            return "x"
        }
        "#,
    );
    assert!(
        errs.iter().any(|m| m.contains("excess: io, audit")),
        "print is Sink+Irreversible → io and audit: {errs:?}"
    );

    // Pure compute carries no effects: ⟨⟩ passes.
    let decls = parse_ok(
        r#"
        pattern Upper(s: String) -> String ⟨⟩ {
            return upper(s)
        }
        "#,
    );
    let errs: Vec<String> = check_program(&decls)
        .errors
        .iter()
        .filter(|e| e.message.contains("effect"))
        .map(|e| e.message.clone())
        .collect();
    assert!(errs.is_empty(), "upper is Pure → no effects: {errs:?}");
}

#[test]
fn n324_memory_statements_are_audit_effects() {
    for body in [
        "memorize \"fact\" with priority=0.5",
        "forget \"query\" after 30.days",
        "relate \"a\" to \"b\" as \"rel\"",
    ] {
        let src = format!("pattern M() -> String ⟨io⟩ {{ {body}\n return \"x\" }}");
        let errs = check_errors(&src);
        assert!(
            errs.iter().any(|m| m.contains("excess: audit")),
            "{body} must be an audit effect: {errs:?}"
        );
    }
}

// ── (г) Recursion: the fixpoint converges without annotations ────────

#[test]
fn n324_recursion_converges_without_annotation() {
    // A directly recursive pattern (no trail): the fixpoint over the
    // 4-element effect lattice must converge (no hang, no error) — the
    // dispatcher's No-Go signal does not fire.
    let decls = parse_ok(
        r#"
        pattern Loop(n: Float) -> String {
            if n > 0.0 {
                return Loop(n - 1.0)
            }
            return "done"
        }
        "#,
    );
    let result = check_program(&decls);
    assert!(
        result.is_ok(),
        "recursive unannotated pattern must infer and pass: {:?}",
        result.format()
    );
}

#[test]
fn n324_recursive_pattern_with_declared_trail_is_gated() {
    // The recursion itself is fine; an explicit trail gets gated against
    // the factual (fixpoint) effects.
    let errs = check_errors(
        r#"
        pattern Loop(n: Float) -> String ⟨⟩ {
            if n > 0.0 {
                return Loop(n - 1.0)
            }
            return "done"
        }
        "#,
    );
    assert!(
        !errs.iter().any(|m| m.contains("effect trail violation")),
        "pure recursion carries no effects: {errs:?}"
    );

    let errs = check_errors(
        r#"
        pattern Loop(n: Float) -> String ⟨⟩ {
            if n > 0.0 {
                print(to_string(n))
                return Loop(n - 1.0)
            }
            return "done"
        }
        "#,
    );
    assert!(
        errs.iter().any(|m| m.contains("excess: io, audit")),
        "recursive body effects must still be gated: {errs:?}"
    );
}

#[test]
fn n324_mutual_recursion_converges() {
    // A → B → A: the fixpoint must stabilize (both bodies are pure).
    let decls = parse_ok(
        r#"
        pattern A(n: Float) -> String {
            if n > 0.0 { return B(n - 1.0) }
            return "a"
        }
        pattern B(n: Float) -> String {
            if n > 0.0 { return A(n - 1.0) }
            return "b"
        }
        "#,
    );
    let result = check_program(&decls);
    assert!(
        result.is_ok(),
        "mutual recursion must converge: {:?}",
        result.format()
    );
}

// ── (д) The showcase example ─────────────────────────────────────────

#[test]
fn n324_example_l1_effect_sig_is_valid_and_gated_clean() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{manifest_dir}/examples/l1_effect_sig.mlog"))
        .expect("example must exist");
    let decls = parse_ok(&src);
    let result = check_program(&decls);
    assert!(
        result.is_ok(),
        "the showcase example must compile clean: {:?}",
        result.format()
    );
    // And it must actually carry trails:
    let has_trail = decls.iter().any(|d| match d {
        Declaration::Pattern(p) => p.effects.is_some(),
        _ => false,
    });
    assert!(has_trail, "the example must declare trails");
}

// ── (е) API surface: Effect::parse_word / format_effect_set ──────────

#[test]
fn n324_effect_words_and_display_are_canonical() {
    assert_eq!(Effect::parse_word("io"), Some(Effect::Io));
    assert_eq!(Effect::parse_word("audit"), Some(Effect::Audit));
    assert_eq!(Effect::parse_word("network"), None);
    assert_eq!(Effect::Io.word(), "io");
    assert_eq!(Effect::Audit.word(), "audit");
    // BTreeSet order is canonical (enum order: Io < Audit).
    let s = effects_of(&["io", "audit"]);
    assert_eq!(metalogos::ast::format_effect_set(&s), "⟨io, audit⟩");
    assert_eq!(metalogos::ast::format_effect_set(&EffectSet::new()), "⟨⟩");
}

// ── (ж) No-stub hygiene (№16.0-D) ────────────────────────────────────

#[test]
fn n324_no_stub_markers_in_touched_files() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in ["src/semantic.rs", "src/ast.rs", "src/parser/decl.rs"] {
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
