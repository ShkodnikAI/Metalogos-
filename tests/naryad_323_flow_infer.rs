//! Наряд №323 (issue #417) — statement-level label inference over the
//! ADR-0154 lattice (semantic::infer_pattern_labels).
//!
//! Tests:
//! - (а) `examples/l1_flow_infer.mlog`: the private label reaches the
//!   output through if/else + each WITHOUT a single annotation (task 4);
//! - (б) per-kind contracts: LetBinding/Assign (RHS label, replace
//!   semantics), Each/EachWithIndex (iterator = iterable, body cannot
//!   raise it, exit join), While (bounded fixpoint), IfElseBlock/IfThen
//!   (componentwise merge, one-sided assignment conservative),
//!   Return/ExprStmt (output join), Match (arms + scrutinee accounting);
//! - (в) redact lowers `private` but not `poisoned` (ADR-0136 D2 bridge);
//! - (г) no-stub hygiene on the naryad's new files (№16.0-D).
//!
//! Contract table (10/10 kinds): REFERENCE §2.2 + ADR-0154 Appendix A.
//! Boundary (loud): recursion → №324; label polymorphism → deferred;
//! media-handle flows → Phase 2.

use metalogos::ast::{Declaration, PatternDecl};
use metalogos::labels::{Conf, Integrity, Label};
use metalogos::parser::parse;
use metalogos::semantic::infer_pattern_labels;

fn pattern_from(source: &str, name: &str) -> PatternDecl {
    let decls = parse(source).unwrap_or_else(|e| panic!("parse failed: {e}"));
    for d in decls {
        if let Declaration::Pattern(p) = d {
            if p.name == name {
                return p;
            }
        }
    }
    panic!("pattern {name} not found");
}

fn conf_of(inf: &metalogos::semantic::LabelInference, var: &str) -> Conf {
    inf.var_labels
        .get(var)
        .unwrap_or_else(|| panic!("var {var} missing"))
        .conf
}

// ── (а) The example: label arrives through if+each, no annotations ──

#[test]
fn n323_example_label_arrives_through_if_and_each() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/l1_flow_infer.mlog"
    ))
    .expect("example must exist");
    let decls = parse(&src).expect("example must parse");
    let mut route: Option<PatternDecl> = None;
    for d in decls {
        if let Declaration::Pattern(p) = d {
            if p.name == "Route" {
                route = Some(p);
            }
        }
    }
    let route = route.expect("Route pattern");
    // No annotation anywhere: purity of the demonstration.
    for prm in &route.params {
        assert!(prm.label.is_none(), "the example must not use annotations");
    }
    let inf = infer_pattern_labels(&route);
    // env("L1_DEMO_KEY") inside the then-branch → private arrives through
    // the if/else merge and the each exit join.
    assert_eq!(conf_of(&inf, "chosen"), Conf::Private);
    assert_eq!(conf_of(&inf, "gathered"), Conf::Private);
    assert_eq!(
        inf.output_label.conf,
        Conf::Private,
        "output label must be private"
    );
    // Secret source: integrity stays trusted.
    assert_eq!(inf.var_labels["chosen"].integrity, Integrity::Trusted);
}

// ── (б1) LetBinding / Assign — label from RHS, Assign replaces ──────

#[test]
fn n323_letbinding_takes_rhs_label() {
    let p = pattern_from("pattern P() -> String { let x = env(\"K\") return x }", "P");
    let inf = infer_pattern_labels(&p);
    assert_eq!(conf_of(&inf, "x"), Conf::Private);
    assert_eq!(inf.output_label.conf, Conf::Private);
}

#[test]
fn n323_assign_replaces_label_in_straight_line() {
    // Straight-line reassignment to a safe value lowers the label —
    // mirrors TaintTracker untaint semantics (merge points re-add
    // conservatism).
    let p = pattern_from(
        "pattern P() -> String { let x = env(\"K\") x = \"safe\" return x }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(conf_of(&inf, "x"), Conf::Public);
    assert_eq!(inf.output_label.conf, Conf::Public);
}

// ── (б2) Each / EachWithIndex ────────────────────────────────────────

#[test]
fn n323_each_iterator_is_iterable_and_body_cannot_raise_it() {
    // xs is unannotated → bottom; body assigns the iterator from a secret
    // — after the loop the iterator keeps the ITERABLE's label.
    let p = pattern_from(
        "pattern P(xs: List) -> String { let mut acc = \"\" each it in xs { it = env(\"K\") acc = acc + it } return acc }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(
        conf_of(&inf, "it"),
        Conf::Public,
        "body must not raise the iterator"
    );
    // The secret flows into acc within the body: exit join keeps it private.
    assert_eq!(conf_of(&inf, "acc"), Conf::Private);
}

#[test]
fn n323_each_exit_joins_iterator_label_into_body_assignments() {
    // Private iterable: acc = acc + it → private arrives via the exit join.
    let p = pattern_from(
        "pattern P(xs: List<private>) -> String { let mut acc = \"\" each it in xs { acc = acc + it } return acc }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(conf_of(&inf, "it"), Conf::Private);
    assert_eq!(conf_of(&inf, "acc"), Conf::Private);
}

#[test]
fn n323_each_with_index_index_var_is_bottom() {
    let p = pattern_from(
        "pattern P(xs: List<private>) -> String { let mut acc = \"\" each i, it in xs { acc = acc + it } return acc }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    // The index is a position, not data: bottom even for a private list.
    assert_eq!(conf_of(&inf, "i"), Conf::Public);
    assert_eq!(inf.var_labels["i"].integrity, Integrity::Trusted);
    assert_eq!(conf_of(&inf, "it"), Conf::Private);
}

// ── (б3) While — bounded fixpoint ────────────────────────────────────

#[test]
fn n323_while_fixpoint_accumulates_body_label() {
    // Loop-carried growth: s feeds itself across iterations; the bounded
    // fixpoint must converge to private.
    let p = pattern_from(
        "pattern P() -> String { let mut s = \"\" while 1.0 < 2.0 { s = s + env(\"K\") } return s }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(conf_of(&inf, "s"), Conf::Private);
    assert_eq!(inf.output_label.conf, Conf::Private);
}

#[test]
fn n323_while_two_var_chain_converges() {
    // a = b; b = secret — needs a second pass to saturate (the chain
    // crosses one iteration boundary).
    let p = pattern_from(
        "pattern P() -> String { let mut a = \"\" let mut b = \"\" while 1.0 < 2.0 { a = b b = env(\"K\") } return a }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(
        conf_of(&inf, "a"),
        Conf::Private,
        "two-var loop-carried chain must saturate"
    );
}

// ── (б4) If/else merges — componentwise, one-sided conservative ─────

#[test]
fn n323_ifelse_join_of_branches() {
    let p = pattern_from(
        "pattern P(mode: String) -> String { let mut v = \"\" if mode == \"a\" then { v = env(\"K\") } else { v = \"pub\" } return v }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(
        conf_of(&inf, "v"),
        Conf::Private,
        "join(then=private, else=public) = private"
    );
}

#[test]
fn n323_one_sided_assignment_is_conservative() {
    // Assigned in the then-branch only: the else path contributes the
    // entry label — merge = join(entry, then).
    let p = pattern_from(
        "pattern P(mode: String) -> String { let mut v = \"\" if mode == \"a\" then { v = env(\"K\") } return v }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(conf_of(&inf, "v"), Conf::Private);
}

// ── (б5) Return / ExprStmt — output join ─────────────────────────────

#[test]
fn n323_return_and_exprstmt_join_output() {
    let p = pattern_from(
        "pattern P(mode: String) -> String { if mode == \"a\" then { return env(\"K\") } respond(\"ok\") return \"done\" }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(
        inf.output_label.conf,
        Conf::Private,
        "return(env) and the ExprStmt path join"
    );
}

// ── (б6) Match — arms join + scrutinee accounting ────────────────────

#[test]
fn n323_match_joins_arms() {
    let p = pattern_from(
        "pattern P(mode: String) -> String { let mut v = \"\" match mode { \"a\" then { v = env(\"K\") } \"b\" then { v = \"pub\" } } return v }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(conf_of(&inf, "v"), Conf::Private, "join over arms");
}

#[test]
fn n323_match_scrutinee_label_joins_assigned_vars() {
    // The scrutinee itself is private; the arm assigns a literal —
    // control dependence on private data taints the outcome.
    let p = pattern_from(
        "pattern P(secret_data: String<private>) -> String { let mut v = \"\" match secret_data { \"x\" then { v = \"derived\" } } return v }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(
        conf_of(&inf, "v"),
        Conf::Private,
        "scrutinee label must join assigned vars"
    );
}

// ── (в) redact: lowers private, never poisoned (ADR-0136 D2 bridge) ──

#[test]
fn n323_redact_lowers_private_but_not_poisoned() {
    let p = pattern_from(
        "pattern P() -> String { let s = redact(env(\"K\"), \"secrets\") return s }",
        "P",
    );
    let inf = infer_pattern_labels(&p);
    assert_eq!(
        conf_of(&inf, "s"),
        Conf::Public,
        "redact(secrets) masks private → public"
    );

    // Poisoned is not curable: a quarantined value stays poisoned even
    // under redact (the channel is not a secret — ADR-0136 D2). The
    // poisoned entry point in this slice is the annotation (№322);
    // statement-level canary poisoning is audit.rs's path-sensitive rule.
    let p2 = pattern_from(
        "pattern P2(leaked: String<poisoned>) -> String { let s = redact(leaked, \"all\") return s }",
        "P2",
    );
    let inf2 = infer_pattern_labels(&p2);
    assert_eq!(
        conf_of(&inf2, "s"),
        Conf::Poisoned,
        "redact must not cure quarantine"
    );
    assert_eq!(
        inf2.output_label.conf,
        Conf::Poisoned,
        "no legal sinks for poisoned (№325)"
    );
}

// ── (г) No-stub hygiene on the naryad's new files ────────────────────

#[test]
fn n323_no_stub_markers_in_new_files() {
    for file in [
        "examples/l1_flow_infer.mlog",
        "tests/naryad_323_flow_infer.rs",
    ] {
        let src = std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), file))
            .unwrap_or_else(|_| panic!("{file} must exist"));
        // Markers built without literals so this check can grep its own file.
        let markers = ["todo", "unimplemented"].map(|m| format!("{m}!"));
        let markers: Vec<String> = markers
            .into_iter()
            .chain(["SK".to_string() + "ELETON"])
            .collect();
        for marker in markers {
            assert!(
                !src.contains(&marker),
                "stub marker `{marker}` found in {file}"
            );
        }
    }
}

// Silence the unused-import warning path if Label import is only used in
// one cfg — keep the API surface stable.
#[test]
fn n323_label_type_is_reexported_stable() {
    let l = Label::bottom();
    assert_eq!(l.conf, Conf::Public);
}
