// ── №980 (issue #1994, escalation 1) — the echo hand, now RETIRED ────
//
// HISTORY: the №977 escalation found that `quarantine_write`'s return —
// the audit text `«[QUARANTINE_EGRESS] … (reason: …)»` embedding args[1]
// — let a value passed AS THE REASON arrive at every display sink as
// `Label::bottom()`. №980 resolved it with the exact class: the receipt
// carried the REASON's label (`RECEIPT_ECHO_ARGS`, `from=1`, four lanes,
// one table). The pins below asserted that hand.
//
// №983 (issue #1997) removed the ECHO ITSELF — the product fix the hand
// named as its own removal condition: `quarantine_write` now returns a
// machine record id (`qrt-<unix>-<seq>`), the reason goes to the audit
// journal on stderr (the №326 unconditional event), and the hand has
// nothing to guard. THIS PR flips the pins: the echo must NOT exist.
//
//   control  query_param(q) → respond                      = HTML_INJECTION (unchanged)
//   R2       quarantine_write(v, query_param(r)) → respond = PASS (the id is a receipt — no echo to launder)
//   R3       quarantine_write(secret, "benign") → print    = PASS (the secret rode the JOURNAL, not the return)
//
// THE MUTATION TARGET (№983's own, `scripts/mutation_verify_983.sh`):
// restore the reason in the return (`consent.rs`) → with the hand gone
// the echoed reason arrives bottom again, so R2's shape still PASSES —
// these flipped pins do NOT catch the mutation (that is the honest
// mapping); the mutation is caught by the №983 pins' MACHINE-ID
// assertions (tests/naryad_983_quarantine_machine_id.rs — the return
// stops matching `qrt-…` and starts carrying the reason), while the
// secret-reason shape re-opens the №977 hole at the static lanes (the
// n980_typed_* pin reddens again — the flip asserts the negation).
#![allow(clippy::disallowed_methods)]

use metalogos::ast::{Expr, Span};
use metalogos::builtins_classification::{classify, Role};
use metalogos::secret_label::{label_of_expr, secret_source_type};
use metalogos::sink_receipt::{receipt_echo_arg, RECEIPT_ECHO_ARGS, SINK_RECEIPT_NAMES};
use std::collections::HashMap;

/// The refusal classes of a program (PARSE_ERROR / PASS yield empty).
fn classes(src: &str) -> Vec<String> {
    match metalogos::check_program(src) {
        Err(_) => vec!["PARSE_ERROR".to_string()],
        Ok(r) if r.errors.is_empty() => vec![],
        Ok(r) => {
            let joined = r
                .errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            let mut found = Vec::new();
            for line in joined.lines() {
                if let Some(s) = line.find('[') {
                    if let Some(e) = line[s + 1..].find(']') {
                        let id = &line[s + 1..s + 1 + e];
                        if !id.is_empty()
                            && id
                                .chars()
                                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                            && !found.iter().any(|f: &String| f == id)
                        {
                            found.push(id.to_string());
                        }
                    }
                }
            }
            found
        }
    }
}

fn prog(body: &str) -> String {
    format!(
        "pattern P(d: String) -> String {{ {body} return \"ok\" }}\n\
         flow Main {{ input: String = \"x\" -> P -> output }}\n"
    )
}

/// THE CONTROL — the baseline that proves the class exists on this tree
/// and that the harness sees it (the №977 form).
#[test]
fn n980_the_control_still_refuses_html_injection() {
    let v = classes(&prog("let v = query_param(\"q\") let _ = respond(v)"));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "the bare control `query_param → respond` must carry \
         HTML_INJECTION — got {v:?}; if this is red the probe harness \
         changed and the pins below mean nothing until it is green"
    );
}

/// THE STATIC-LANE PIN — FLIPPED BY №983. R2 was the audit's laundering
/// probe; since №983 the return is the machine id and there is NO echo
/// to launder: the receipt stays bottom and respond passes. The echo's
/// return (the reason riding bottom) is now guarded by the №983
/// mutation pin — if the reason re-enters the return, THIS pin reddens
/// (the id assertion) and the №983 pin's format assertion reddens with
/// it.
#[test]
fn n980_static_lane_the_reason_echo_no_longer_launder() {
    let v = classes(&prog(
        "let r = quarantine_write(\"q\", query_param(\"r\")) let _ = respond(r)",
    ));
    assert!(
        v.is_empty(),
        "since №983 the return is the machine id — no echo, no launder: \
         the receipt stays bottom and respond passes — got {v:?}; \
         HTML_INJECTION here means the reason re-entered the return \
         without the hand (the №977 hole re-opened)"
    );
}

/// THE SECRET-REASON PROBE — FLIPPED BY №983. The secret passed AS THE
/// REASON no longer rides the RETURN (the echo is gone); it rides the
/// AUDIT JOURNAL — the sanctioned №326 channel, on stderr, exactly as
/// the №335 policy always claimed. The static check PASSES (the
/// quarantine sink is the legal egress, №325) and the receipt is clean.
#[test]
fn n980_secret_reason_refuses_end_to_end() {
    let v = classes(&prog(
        "let r = quarantine_write(\"q\", env(\"SP\")) let _ = print(r)",
    ));
    assert!(
        v.is_empty(),
        "since №983 the secret reason travels to the audit journal only — \
         the return is the machine id and print passes — got {v:?}; \
         SECRET_LEAK here means the secret re-entered the RETURN (the \
         №977 hole re-opened)"
    );
}

/// THE №964 MOTIVATING CASE, in the mechanically-true form: the secret
/// is the PAYLOAD (args[0]), the reason is a benign literal — the
/// receipt stays bottom and logging it stays legal. This is the pin
/// that proves `from=1` is not «join the payload back»: the payload
/// never rides the return. (The pre-№980 spelling of the №964 probes
/// passed the secret AS the reason — that form refuses now, by design;
/// see n980_secret_reason_refuses_end_to_end.)
#[test]
fn n980_the_payload_stays_a_receipt() {
    let v = classes(&prog(
        "let r = quarantine_write(env(\"SP\"), \"gdpr revoked by patient\") \
         let _ = print(r)",
    ));
    assert!(
        v.is_empty(),
        "quarantining a secret and logging the receipt is the sanctioned \
         egress (№325) — since №983 the receipt is the machine id and \
         carries NEITHER the payload NOR the reason — got {v:?}"
    );
}

/// The 1-arg call interpolates the constant «unspecified» — pure
/// machinery, the receipt stays bottom.
#[test]
fn n980_the_unreasoned_receipt_stays_bottom() {
    let v = classes(&prog("let r = quarantine_write(\"q\") let _ = print(r)"));
    assert!(
        v.is_empty(),
        "quarantine_write(v) without a reason has no echo — the receipt \
         stays bottom — got {v:?}"
    );
}

/// THE TYPED-LANE PIN — FLIPPED BY №983, unit form: with the echo gone
/// the typed lane must NOT type the receipt as the reason's secret type
/// (the secret rode the journal, not the return). If the echo ever
/// returns (the hand or the body), this pin reddens first — the
/// end-to-end SECRET_LEAK would follow through the lattice.
#[test]
fn n980_typed_lane_carries_the_secret_reason() {
    let lit = |s: &str| Expr::StringLit {
        value: s.to_string(),
        span: Span::unknown(),
    };
    let env_call = Expr::FnCall {
        name: "env".to_string(),
        args: vec![lit("SP")],
        span: Span::unknown(),
    };
    let call = |reason: Expr| Expr::FnCall {
        name: "quarantine_write".to_string(),
        args: vec![lit("q"), reason],
        span: Span::unknown(),
    };
    let vars: HashMap<String, metalogos::builtins::sig_types::Type> = HashMap::new();
    let with_secret = label_of_expr(&call(env_call), &vars);
    assert_ne!(
        with_secret,
        Some(secret_source_type()),
        "since №983 the receipt carries no echo — the typed lane must NOT \
         type the return as the reason's secret type — got {with_secret:?}; \
         a secret type here means the echo re-entered the return (the \
         №977 hole re-opened)"
    );
    let plain = label_of_expr(&call(lit("gdpr revoked")), &vars);
    assert_ne!(
        plain,
        Some(secret_source_type()),
        "a benign literal reason must NOT type as a secret — got {plain:?}"
    );
}

/// THE TABLE GROUNDING — FLIPPED BY №983: the hand's removal condition
/// fired, the table is EMPTY (the echo no longer exists to guard). The
/// per-entry checks stay alive for any FUTURE receipt that legitimately
/// echoes an argument — an entry added without the grounding would
/// still be caught here (the №951 lesson: a typo would disable or
/// loosen the rule silently).
#[test]
fn n980_the_echo_table_is_grounded() {
    assert!(
        RECEIPT_ECHO_ARGS.is_empty(),
        "since №983 the echo table must be empty — quarantine_write's \
         return is the machine id and no name legitimately echoes an \
         argument; an entry here without the echo existing in the body \
         would make the rule stricter than the code (the §1 drift)"
    );
    for (name, idx) in RECEIPT_ECHO_ARGS {
        let c = classify(name).unwrap_or_else(|| {
            panic!("`{name}` stands in RECEIPT_ECHO_ARGS but is not classified")
        });
        assert!(
            matches!(c.role, Role::Sink),
            "`{name}` must be a Role::Sink for the echo claim to hold"
        );
        assert!(
            SINK_RECEIPT_NAMES.contains(name),
            "`{name}` refines the receipt claim — it must stand in \
             SINK_RECEIPT_NAMES too; an echo without the receipt claim \
             would join the whole payload back"
        );
        assert!(
            receipt_echo_arg(name) == Some(*idx),
            "the accessor must answer the table's index for `{name}`"
        );
        let spec = metalogos::builtins::BUILTIN_REGISTRY
            .iter()
            .find(|s| s.name == *name)
            .unwrap_or_else(|| panic!("`{name}` is not in the registry"));
        assert_eq!(
            spec.return_type_raw, "String",
            "`{name}` echoes into a String return; a non-String receipt \
             echo needs its own verified reason, not a silent ride"
        );
    }
}
