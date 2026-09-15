//! Наряд №322 (issue #416) — ADR-0154: label lattice `(conf, integrity,
//! consent-scope)` + label carrier in the AST + annotation syntax.
//!
//! Tests:
//! - (а) annotation syntax: `String<private>` / `String<private,
//!   untrusted, consent(gdpr)>` parses on pattern params and entity
//!   declarations; `type_name` stays the bare type; the raw annotation
//!   lands in `ast::LabelAnn` with a span (task 2);
//! - (б) componentwise join/meet incl. poisoned absorbing for BOTH
//!   (quarantine is not curable by meet — ADR-0154 D1);
//! - (в) semantic visibility: unknown label words are loud errors with
//!   the annotation's span; valid annotations introduce no new
//!   diagnostics (task 2);
//! - (г) legacy `TaintKind` projection — additive mapping, kinds kept
//!   (task 3; exhaustiveness pinned inside `src/audit.rs` tests);
//! - (д) no-stubs hygiene: the new module has no `todo!` /
//!   `unimplemented!` / `SKELETON` (№16.0-D).
//!
//! Boundary (loud, per the naryad): statement-level label inference is
//! №323, pattern effect-trail №324, sink-gate №325 — NOT tested here.

use metalogos::ast::{Declaration, LabelAnn};
use metalogos::labels::{legacy_taint_label, Conf, ConsentScope, Integrity, Label};
use metalogos::parser::parse;
use metalogos::semantic::check_program;

fn parse_ok(source: &str) -> Vec<Declaration> {
    parse(source).unwrap_or_else(|e| panic!("parse failed: {e}"))
}

// ── (а) Annotation syntax on the label carrier ──────────────────────

#[test]
fn n322_param_annotation_parses_and_carries_label() {
    let decls = parse_ok(
        r#"
        pattern Greet(name: String<private>) -> String {
            return "hi, " + name
        }
        "#,
    );
    assert_eq!(decls.len(), 1);
    let Declaration::Pattern(p) = &decls[0] else {
        panic!("expected pattern");
    };
    assert_eq!(p.params.len(), 1);
    let prm = &p.params[0];
    // The type name stays bare — semantic type checks are unaffected.
    assert_eq!(prm.type_name, "String");
    let ann = prm.label.as_ref().expect("label must be carried");
    assert_eq!(ann.raw, "private");
    // Span points at the annotation in source.
    assert!(ann.span.start_line > 0, "annotation must carry a span");
}

#[test]
fn n322_full_three_component_annotation_parses() {
    let decls = parse_ok(
        r#"
        pattern Share(data: String<consented, untrusted, consent(gdpr, analytics)>) -> String {
            return data
        }
        "#,
    );
    let Declaration::Pattern(p) = &decls[0] else {
        panic!("expected pattern");
    };
    let ann: &LabelAnn = p.params[0].label.as_ref().expect("label must be carried");
    assert_eq!(ann.raw, "consented, untrusted, consent(gdpr, analytics)");
    // And it is semantically valid — no errors.
    let result = check_program(&decls);
    assert_eq!(
        result
            .errors
            .iter()
            .filter(|e| e.message.contains("label"))
            .count(),
        0
    );
}

#[test]
fn n322_entity_annotations_parse_on_both_entity_forms() {
    let decls = parse_ok(
        r#"
        entity Person { text: String<private>, nick: String }
        entity secret_k: String<private> = env("K")
        entity audit_log: String<consented, trusted, consent(gdpr)> = ""
        "#,
    );
    let mut seen_record = false;
    let mut seen_simple = 0;
    for d in &decls {
        match d {
            Declaration::EntityType(e) => {
                let labeled: Vec<&str> = e
                    .fields
                    .iter()
                    .filter(|f| f.label.is_some())
                    .map(|f| f.name.as_str())
                    .collect();
                assert_eq!(
                    labeled,
                    vec!["text"],
                    "only the private field carries a label"
                );
                seen_record = true;
            }
            Declaration::EntitySimple(e) => {
                assert!(e.label.is_some(), "simple entity annotation lost");
                seen_simple += 1;
            }
            _ => {}
        }
    }
    assert!(seen_record);
    assert_eq!(seen_simple, 2);
}

#[test]
fn n322_programs_without_annotations_unchanged() {
    // No label anywhere → Param.label is None; no new diagnostics.
    let decls = parse_ok(
        r#"
        entity greeting: String = "Hello"
        pattern SayHello(text: String) -> String { return text }
        "#,
    );
    let Declaration::Pattern(p) = &decls[1] else {
        panic!("expected pattern");
    };
    assert!(p.params[0].label.is_none());
    let result = check_program(&decls);
    assert!(
        result.is_ok(),
        "plain programs must stay clean: {:?}",
        result.format()
    );
}

// ── (б) Componentwise join/meet, poisoned quarantine ─────────────────

#[test]
fn n322_join_meet_componentwise_and_poisoned_absorbing() {
    let public_trusted = Label {
        conf: Conf::Public,
        integrity: Integrity::Trusted,
        consent: ConsentScope::new(),
    };
    let private_untrusted = Label {
        conf: Conf::Private,
        integrity: Integrity::Untrusted,
        consent: ConsentScope::from_scopes(["gdpr"]),
    };
    // join (data combination): max conf, min integrity, intersected consent.
    let j = public_trusted.join(&private_untrusted);
    assert_eq!(j.conf, Conf::Private);
    assert_eq!(j.integrity, Integrity::Untrusted);
    assert!(j.consent.is_empty());
    // meet (requirement combination): min conf, max integrity, unioned consent.
    let m = public_trusted.meet(&private_untrusted);
    assert_eq!(m.conf, Conf::Public);
    assert_eq!(m.integrity, Integrity::Trusted);
    assert_eq!(m.consent.scopes().len(), 1);

    // Quarantine: poisoned absorbs join AND meet — no legal sinks, and
    // meet is NOT a declassifier (ADR-0154 D1).
    let poisoned = Label {
        conf: Conf::Poisoned,
        integrity: Integrity::Untrusted,
        consent: ConsentScope::new(),
    };
    for other in [&public_trusted, &private_untrusted] {
        assert_eq!(poisoned.join(other).conf, Conf::Poisoned);
        assert_eq!(other.join(&poisoned).conf, Conf::Poisoned);
        assert_eq!(poisoned.meet(other).conf, Conf::Poisoned);
        assert_eq!(other.meet(&poisoned).conf, Conf::Poisoned);
    }
}

// ── (в) Semantic visibility of annotations ───────────────────────────

#[test]
fn n322_unknown_label_word_is_loud_semantic_error() {
    let decls = parse_ok(
        r#"
        pattern P(x: String<secretive>) -> String { return x }
        "#,
    );
    let result = check_program(&decls);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("unknown label word 'secretive'")),
        "semantic must reject unknown label words, got: {:?}",
        result.format()
    );
}

#[test]
fn n322_label_errors_carry_the_annotation_span() {
    let decls = parse_ok(
        r#"
        entity k: String<bogus> = "v"
        "#,
    );
    let result = check_program(&decls);
    let err = result
        .errors
        .iter()
        .find(|e| e.message.contains("label"))
        .expect("label error expected");
    assert_eq!(
        err.span.start_line, 2,
        "error must point at the annotation line"
    );
}

#[test]
fn n322_missing_conf_word_is_loud() {
    // Bare `<untrusted>` — conf is REQUIRED (ADR-0154 §3): the silent
    // default "public" would be the wrong (loudest) default to hide.
    let decls = parse_ok(
        r#"
        pattern P(x: String<untrusted>) -> String { return x }
        "#,
    );
    let result = check_program(&decls);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("no confidentiality word")),
        "missing conf word must be a loud error: {:?}",
        result.format()
    );
}

#[test]
fn n322_duplicate_components_are_loud() {
    let decls = parse_ok(
        r#"
        pattern P(x: String<private, private>) -> String { return x }
        "#,
    );
    let result = check_program(&decls);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("duplicate confidentiality word")),
        "duplicate conf must be a loud error: {:?}",
        result.format()
    );
}

// ── (г) Legacy TaintKind projection ─────────────────────────────────

#[test]
fn n322_legacy_taint_projection_is_additive_and_total() {
    // The five legacy kinds project; unknown names do not.
    for kind in [
        "LlmOutput",
        "Secret",
        "UserInput",
        "Sanitized",
        "CanaryLeak",
    ] {
        let l = legacy_taint_label(kind).unwrap_or_else(|| panic!("{kind} must project"));
        assert!(!l.to_string().is_empty());
    }
    assert!(legacy_taint_label("Nonexistent").is_none());

    // Spot-check the semantic of the mapping (ADR-0154 §5).
    assert_eq!(legacy_taint_label("Secret").unwrap().conf, Conf::Private);
    assert_eq!(
        legacy_taint_label("CanaryLeak").unwrap().conf,
        Conf::Poisoned
    );
    assert_eq!(
        legacy_taint_label("LlmOutput").unwrap().integrity,
        Integrity::Untrusted
    );
    assert_eq!(
        legacy_taint_label("Sanitized").unwrap().integrity,
        Integrity::Trusted
    );
}

// ── (д) No-stub hygiene (№16.0-D) ────────────────────────────────────

#[test]
fn n322_labels_module_has_no_stub_markers() {
    // The module source must not contain stub markers — mirrors the
    // dispatcher's grep rule so the contract holds in CI.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{manifest_dir}/src/labels.rs"))
        .expect("labels.rs must exist");
    for marker in ["todo!", "unimplemented!", "SKELETON"] {
        assert!(
            !src.contains(marker),
            "stub marker `{marker}` found in src/labels.rs"
        );
    }
}
