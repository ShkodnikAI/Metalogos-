// ── №950 (issue #1917): the interprocedural summaries (ADR-0154 §12) ──
//
// The lattice shipped through v0.39.0 was INTRAPROCEDURAL. A call to a
// user pattern took the join of its ARGUMENTS and never consulted the
// called body; an unannotated parameter entered that body as `bottom` —
// the «open world» of ADR-0154 Appendix A. The audit of 12.11 proved both
// halves on the PUBLISHED release binary: a source inside a called pattern
// reached a sink at the call site with `mlog check` clean, and a parameter
// reaching a sink inside the body leaked whatever the caller handed it.
//
// §12.2 closes both with one summary per pattern:
//   (а) the return label joins the sources evaluated INSIDE the body;
//   (б) each parameter carries the clearance of the sinks it reaches, and
//       the CALL SITE checks the actual argument against it.
//
// The four pin rows B1–B4 of the threat-model boundary table are flipped
// in tests/naryad_948_boundary_pins.rs. This file pins the properties the
// pins cannot express on their own: DEPTH (a chain of any length, not just
// one hop), CYCLES (the conservative top), the ABSENCE of false positives,
// and the §12.5 transition window.

fn compile_errors(src: &str) -> Vec<String> {
    match metalogos::check_program(src) {
        Ok(result) => result
            .errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>(),
        Err(e) => panic!("the fixture itself must parse: {e}"),
    }
}

/// A chain of `n` forwarding patterns ending in a `print` of its argument,
/// with a secret handed in at the top.
fn param_chain(n: usize) -> String {
    let mut lines = vec!["pattern L0(v: String) -> String { print(v) return \"ok\" }".to_string()];
    for i in 1..=n {
        lines.push(format!(
            "pattern L{i}(v: String) -> String {{ return L{prev}(v) }}",
            prev = i - 1
        ));
    }
    lines.push(format!(
        "pattern Top(x: String) -> String {{ return L{n}(env(\"SP\")) }}"
    ));
    lines.push("flow Main { input: String = \"x\" -> Top -> output }".to_string());
    lines.join("\n")
}

/// A chain of `n` forwarding patterns returning a secret read at the
/// bottom, printed at the top.
fn return_chain(n: usize) -> String {
    let mut lines = vec!["pattern R0(x: String) -> String { return env(\"SP\") }".to_string()];
    for i in 1..=n {
        lines.push(format!(
            "pattern R{i}(x: String) -> String {{ return R{prev}(x) }}",
            prev = i - 1
        ));
    }
    lines.push(format!(
        "pattern RTop(x: String) -> String {{ print(R{n}(\"a\")) return \"ok\" }}"
    ));
    lines.push("flow Main { input: String = \"x\" -> RTop -> output }".to_string());
    lines.join("\n")
}

/// DEPTH, half (б): a parameter reaching a sink through a chain of
/// patterns refuses at EVERY depth — one hop and twenty.
///
/// This is the mutation target of the computation ORDER. The first version
/// of this code iterated a capped fixpoint (8 passes): a chain deeper than
/// the cap came out as an UNDER-approximation — the leak at depth 12 was
/// invisible, which is the same silent gap the naryad exists to close. The
/// summaries are computed in reverse topological order instead, so a chain
/// of any depth is complete in one pass. Lower the depths below and the
/// iteration-cap bug comes back visible.
#[test]
fn n950_param_to_sink_refuses_at_every_chain_depth() {
    for depth in [1usize, 2, 5, 12, 20] {
        let errs = compile_errors(&param_chain(depth));
        assert!(
            errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
            "depth {depth}: a secret handed to a parameter that reaches `print` through \
             {depth} forwarding pattern(s) must refuse (ADR-0154 §12.2 (б)); got: {errs:?}"
        );
    }
}

/// DEPTH, half (а): a source evaluated inside the deepest callee rides the
/// return label up the whole chain to the sink at the top.
#[test]
fn n950_internal_source_rides_the_return_at_every_chain_depth() {
    for depth in [1usize, 2, 5, 12, 20] {
        let errs = compile_errors(&return_chain(depth));
        assert!(
            errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
            "depth {depth}: a secret read inside the deepest callee and returned through \
             {depth} pattern(s) into `print` must refuse (ADR-0154 §12.2 (а)); got: {errs:?}"
        );
    }
}

/// CYCLES (§12.3): a recursive pattern cannot be ordered callees-first, so
/// its component is solved by an ASCENDING fixpoint — a parameter of a
/// recursive pattern that reaches a sink still refuses.
#[test]
fn n950_call_cycle_leak_still_refuses() {
    let src = r#"
        pattern Rec(v: String, n: Float) -> String {
            if n > 0.0 { return Rec(v, n - 1.0) }
            print(v)
            return "ok"
        }
        pattern P(x: String) -> String { return Rec(env("SP"), 3.0) }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "a recursive pattern is a call cycle — its summary is the conservative top \
         (ADR-0154 §12.3) and the leak through it must refuse; got: {errs:?}"
    );
}

/// NO FALSE POSITIVES: public data through a chain of patterns stays
/// clean. The summaries raise a call's label only by what the callee
/// actually carries — the join is monotone, not blanket.
///
/// The whole 386-file `examples/` corpus is the wide version of this
/// assertion: `mlog check` over it changed on exactly the four pin files
/// B1–B4 and nowhere else (the naryad's verification log).
#[test]
fn n950_public_data_through_patterns_stays_clean() {
    let src = r#"
        pattern Up(s: String) -> String { print(s) return s }
        pattern Mid(s: String) -> String { return Up(s) }
        pattern P(x: String) -> String { return Mid("hello") }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "a literal travelling through three patterns into `print` must stay clean — \
         the summaries must not refuse by the SHAPE of a call; got: {errs:?}"
    );
}

/// The §12.5 TRANSITION WINDOW: `profile legacy { interproc: warn }`
/// downgrades exactly the findings the summaries ADDED — the program
/// compiles, and the audit report still records the event, so the consumer
/// is told what will block when the window closes.
#[test]
fn n950_transition_window_compiles_and_still_records_the_event() {
    let src = r#"
        profile legacy { interproc: warn }
        pattern Show(v: String) -> String { print(v) return "ok" }
        pattern P(x: String) -> String { return Show(env("SP")) }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "the №950 transition window must keep compilation green for ONE release window \
         (ADR-0154 §12.5); got: {errs:?}"
    );
    let report = metalogos::audit_program(src).expect("audit must parse");
    let windowed: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.check_id == "PII_EGRESS_OUTPUT")
        .collect();
    assert!(
        !windowed.is_empty(),
        "the window downgrades the finding — it must NOT delete it: the audit report \
         must still carry the interprocedural event (§12.5)"
    );
    assert!(
        windowed
            .iter()
            .any(|f| f.message.contains("interproc warn")),
        "the windowed event must name the window, so the consumer knows it blocks when \
         the window closes; got: {:?}",
        windowed.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
}

/// The window is NARROW: it downgrades the interprocedural findings only.
/// An INTRAprocedural leak in the same program keeps blocking — the window
/// is not a second `egress: permissive_with_audit`.
#[test]
fn n950_transition_window_does_not_downgrade_intraprocedural_refusals() {
    let src = r#"
        profile legacy { interproc: warn }
        pattern Direct(x: String) -> String { print(env("SP")) return "ok" }
        flow Main { input: String = "x" -> Direct -> output }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "`interproc: warn` must downgrade ONLY what the summaries added — a leak inside \
         one pattern keeps blocking (§12.5); got: {errs:?}"
    );
}

/// The profile key is validated like every other one (the №954 form): the
/// one legal mode is `warn`, anything else is a loud refusal rather than a
/// silently ignored option.
#[test]
fn n950_profile_interproc_mode_is_validated() {
    let bad = compile_errors(
        r#"
        profile legacy { interproc: off }
        pattern P(x: String) -> String { return "ok" }
    "#,
    );
    assert!(
        bad.iter().any(|m| m.contains("interproc")),
        "an unknown interproc mode must be a loud refusal, not a silently ignored key; \
         got: {bad:?}"
    );
    let good = compile_errors(
        r#"
        profile legacy { interproc: warn }
        pattern P(x: String) -> String { return "ok" }
    "#,
    );
    assert!(
        good.is_empty(),
        "`interproc: warn` is the legal transition-window mode; got: {good:?}"
    );
}

/// The diagnostic vocabulary stays FROZEN (§12.4): the interprocedural
/// refusals use the existing `check_id`s of the sink-clearance gate — no
/// new class is minted for the boundary.
#[test]
fn n950_no_new_check_id_is_minted() {
    const FROZEN: &[&str] = &[
        "PII_EGRESS_OUTPUT",
        "PII_EGRESS_NETWORK",
        "SECRET_EGRESS_NETWORK",
        "HTML_INJECTION",
        "UNTRUSTED_EXEC_DECISION",
        "SECRET_LEAK",
        "TAINT_PERSISTENCE",
        "SQL_DYNAMIC",
        "UNTRUSTED_FILE_PATH",
        "VOICE_UNCONSENTED",
        "QUARANTINE_EGRESS",
    ];
    for src in [param_chain(3), return_chain(3)] {
        let report = metalogos::audit_program(&src).expect("audit must parse");
        for f in &report.findings {
            assert!(
                FROZEN.contains(&f.check_id),
                "the interprocedural path reported '{}', which is not in the frozen \
                 vocabulary — §12.4 forbids minting a new check_id for the boundary",
                f.check_id
            );
        }
    }
}

/// TOOL METHODS are the same open world — found during the implementation
/// of this naryad, not in the audit's row list, and closed in the same
/// naryad. A `ToolMethod` is structurally a `PatternDecl` (`ast.rs`), so
/// it is summarized by the same machinery under the key `Tool.method`
/// that a `QualifiedCall` resolves to.
///
/// Why it could not wait for its own naryad: with patterns closed and
/// tool methods open, the whole fix is bypassed by wrapping the sink in
/// one `tool { }` block. Both halves are pinned here.
#[test]
fn n950_tool_methods_close_the_same_boundary() {
    let param_half = r#"
        tool T { send(v: String) -> String { print(v) return "ok" } }
        pattern P(x: String) -> String { return T.send(env("SP")) }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(param_half);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "a tool method's parameter reaching a sink must be checked at the call site like a \
         pattern's — otherwise one `tool {{ }}` wrapper bypasses №950 entirely; got: {errs:?}"
    );

    let return_half = r#"
        tool T { get(v: String) -> String { return env("SP") } }
        pattern P(x: String) -> String { print(T.get("a")) return "ok" }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(return_half);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "a source inside a tool method must ride its return label into the caller's sink; \
         got: {errs:?}"
    );

    let clean = r#"
        tool T { send(v: String) -> String { print(v) return "ok" } }
        pattern P(x: String) -> String { return T.send("hello") }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(clean);
    assert!(
        errs.is_empty(),
        "a literal through a tool method must stay clean — the summaries must not refuse by \
         the SHAPE of a qualified call; got: {errs:?}"
    );
}

/// THREE RULE FAMILIES cross the boundary, each re-decided at the call
/// site by the rule that refused inside — not by a single catch-all. The
/// first implementation propagated everything through the generic
/// clearance rule, which named the WRONG rule for two of these: `str` is
/// `Role::Pure` and `read_file` is `Role::Source`, so a clearance
/// re-decision would have reported them as network egress.
#[test]
fn n950_each_rule_family_is_redecided_by_its_own_rule() {
    // (1) the duty contour: lifting a private value into text, through a
    // callee. `str` is not a sink — the duty rule is its own.
    let duty = r#"
        profile duty { materialization: denied }
        pattern Fmt(v: String) -> String { return str(v) }
        pattern P(x: String) -> String { return Fmt(env("SP")) }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(duty);
    assert!(
        errs.iter().any(|m| m.contains("[DUTY_MATERIALIZATION]")),
        "the duty materialization rule must cross the boundary under ITS OWN class — not as a \
         clearance failure on `str`; got: {errs:?}"
    );

    // (2) Н455: an untrusted path choosing which bytes enter the program,
    // through a callee. `read_file` is a Source — the path rule is its own.
    let path = r#"
        pattern Load(p: String) -> String { return read_file(p) }
        mlogserver { port: 18204
          route "/a" method=GET { return respond(Load(query_param("f"))) } }
    "#;
    let errs = compile_errors(path);
    assert!(
        errs.iter().any(|m| m.contains("[UNTRUSTED_FILE_PATH]")),
        "the untrusted-file-path rule must cross the boundary under ITS OWN class; got: {errs:?}"
    );
}

/// `media_save` does NOT cross: its gate reads CALLER-CONTEXT credentials
/// (a consent scope, or a bound likeness token for a camera/likeness
/// origin — №387), which the probe cannot hold, so propagating it would
/// refuse a caller that legitimately holds the credential. The callee's
/// own site keeps the gate — nothing is lost, only not doubled.
#[test]
fn n950_media_save_credentials_are_not_second_guessed_at_the_call_site() {
    let src = r#"
        pattern Save(h: String) -> String { return media_save(h, "out.png") }
        pattern P(x: String) -> String { return Save("data") }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "a public handle through a media_save wrapper must stay clean — the №387 credential \
         gate belongs to its own site; got: {errs:?}"
    );
}

/// The other half of the cycle rule, and the one that cost a regression:
/// a recursion that leaks NOTHING must stay clean. The first version gave
/// every cycle member the conservative TOP, which made the self-hosted
/// parser (`self-host/parser.mlog`) refuse with 23 findings — its mutually
/// recursive parse functions all returned «private, untrusted», so every
/// decision on a parse result became an UNTRUSTED_DECISION. Recursion is
/// ordinary Metalogos; a rule that makes it unusable is a false positive.
///
/// The repo-wide pin for this is the parser itself (0 findings, asserted
/// by `tests/naryad_197_*` and `tests/naryad_816_*`, which run it); this
/// test is the small, fast version of the same property.
#[test]
fn n950_benign_recursion_stays_clean() {
    let mutual = r#"
        pattern ParseA(src: String, n: Float) -> String {
            if n > 0.0 { return ParseB(src, n - 1.0) }
            return "a"
        }
        pattern ParseB(src: String, n: Float) -> String {
            let head = ParseA(src, n - 1.0)
            if head == "a" { return "b" }
            return head
        }
        pattern Run(x: String) -> String { return ParseB("text", 3.0) }
        flow Main { input: String = "x" -> Run -> output }
    "#;
    let errs = compile_errors(mutual);
    assert!(
        errs.is_empty(),
        "mutually recursive patterns that leak nothing must stay clean — the cycle rule must \
         not hand out the conservative TOP by default (the self-host regression); got: {errs:?}"
    );
}

/// A sanitizer INSIDE the callee must be credited for the axis it repairs.
/// `redact(x, "hash_only")` restores the integrity axis and keeps the
/// confidentiality one, so a parameter sanitized that way before a memory
/// sink clears an UNTRUSTED (public) argument at the call site — and still
/// refuses a PRIVATE one.
///
/// This is the mutation target of the per-axis probing. Probe both axes at
/// once and the first assertion fails: the parameter still «reaches» the
/// sink on the confidentiality axis, and the untrusted argument is refused
/// for a reason the callee had already sanitized away. It is also what
/// makes the migration in `docs/upgrade.md` («sanitize inside the callee»)
/// actually work.
#[test]
fn n950_a_sanitizer_inside_the_callee_is_credited_per_axis() {
    let sanitized = r#"
        pattern Store(raw: String) -> String {
            let fp = redact(raw, "hash_only")
            kv_set("k-" + fp, "ok")
            return "stored"
        }
        mlogserver { port: 18208
          route "/s" method=GET { return respond(Store(query_param("id"))) } }
    "#;
    let errs = compile_errors(sanitized);
    assert!(
        !errs.iter().any(|m| m.contains("TAINT_PERSISTENCE")),
        "the callee repairs the INTEGRITY axis before the memory sink, so an untrusted (but \
         public) argument must clear at the call site; got: {errs:?}"
    );

    let unsanitized = r#"
        pattern Store(raw: String) -> String {
            kv_set("k-" + raw, "ok")
            return "stored"
        }
        mlogserver { port: 18209
          route "/s" method=GET { return respond(Store(query_param("id"))) } }
    "#;
    let errs = compile_errors(unsanitized);
    assert!(
        errs.iter().any(|m| m.contains("TAINT_PERSISTENCE")),
        "without the sanitizer the same flow must refuse — otherwise the per-axis probing has \
         simply switched the gate off; got: {errs:?}"
    );

    let private_arg = r#"
        pattern Store(raw: String) -> String {
            let fp = redact(raw, "hash_only")
            kv_set("k-" + fp, "ok")
            return "stored"
        }
        pattern P(x: String) -> String { return Store(env("SP")) }
        flow Main { input: String = "x" -> P -> output }
    "#;
    let errs = compile_errors(private_arg);
    assert!(
        !errs.is_empty(),
        "`hash_only` repairs the integrity, NOT the confidentiality — a secret argument must \
         still refuse at the call site; got a clean check"
    );
}
