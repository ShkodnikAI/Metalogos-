// ── №980 (issue #1994, the escalations line 19.11 — escalation 1):
//    the receipt that ECHOES its reason — `quarantine_write` from=1 ────
//
// THE FACT (the №977 escalation, resolved by the owner's dispatch
// gh#2000): `quarantine_write` STAYED in SINK_RECEIPT_NAMES after №977
// pulled the three echo names out. Its return is the audit text
// `«[QUARANTINE_EGRESS] … (reason: …)»` embedding args[1] — the
// caller-supplied reason (src/builtins/consent.rs, read in the bounds
// of the handler body). A value passed AS THE REASON therefore arrived
// at every display sink as `Label::bottom()` — not conservative, but
// ABSENT — while the bare control refuses:
//
//   control  query_param(q) → respond                      = HTML_INJECTION
//   R2       quarantine_write(v, query_param(r)) → respond = PASS ← the hole
//   R3       quarantine_write(secret, "benign") → print    = PASS ← stays
//
// THE FIX: `from=1` — the receipt carries EXACTLY the label of the
// argument it interpolates (args[1]), never the payload (args[0]).
// The table is `sink_receipt::RECEIPT_ECHO_ARGS`, one table next to the
// receipt and external-return tables, consulted by FOUR lanes in the
// SAME order (after the private-field bridge, before `is_sink_receipt`):
// the static lattice (semantic.rs), the TW runtime (interpreter/labels.rs),
// the VM runtime (vm.rs::builtin_result_label) and the typed secret lane
// (secret_label.rs). One table, no island (§2).
//
// THE REMOVAL CONDITION (№983): when the machine-id product fix lands —
// the reason moves into the №335 audit event — the hand is removed BY
// THAT naryad's PR; the hand outliving the echo it models would make
// the rule stricter than the code (the §1 drift, the reverse direction).
//
// THE MUTATION TARGETS, one per lane, each pin naming its hand: remove
// the echo consult in `semantic.rs` → n980_static_* reddens; in
// `interpreter/labels.rs` → n980_tw_runtime_* reddens; in `vm.rs` →
// n980_vm_runtime_* reddens; in `secret_label.rs` → n980_typed_* reddens.
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

/// THE STATIC-LANE PIN (the lattice hand in `semantic.rs`). R2 — the
/// audit's laundering probe: the reason echo must not arrive at the
/// public channel as bottom. The class is the INTEGRITY one, and the
/// typed lane must NOT raise it (query_param is untrusted, not a
/// secret) — the two lanes agree, the №476 form.
#[test]
fn n980_static_lane_the_reason_echo_no_longer_launder() {
    let v = classes(&prog(
        "let r = quarantine_write(\"q\", query_param(\"r\")) let _ = respond(r)",
    ));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "quarantine_write returns the audit text embedding args[1] (the \
         reason, consent.rs) — the receipt must carry the REASON's label \
         (from=1, №980), so respond refuses like the control — got {v:?}; \
         PASS here is the laundering path the №977 escalation named"
    );
    assert!(
        !v.iter().any(|c| c == "SECRET_LEAK"),
        "query_param is untrusted but NOT a secret — SECRET_LEAK here is \
         a lane disagreement, not a fix — got {v:?}"
    );
}

/// THE SECRET-REASON PROBE (the №980 task 4 form): a SECRET passed AS
/// THE REASON must refuse end-to-end. The typed lane's class is
/// SECRET_LEAK; the lattice agrees (env is Private on both).
#[test]
fn n980_secret_reason_refuses_end_to_end() {
    let v = classes(&prog(
        "let r = quarantine_write(\"q\", env(\"SP\")) let _ = print(r)",
    ));
    assert!(
        v.iter().any(|c| c == "SECRET_LEAK"),
        "a secret passed AS THE REASON rides the receipt echo — print \
         must refuse with SECRET_LEAK (№980 task 4) — got {v:?}"
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
         egress (№325) — the receipt carries the REASON's label only, and \
         the reason is a benign literal — got {v:?}"
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

/// THE TYPED-LANE PIN (the hand in `secret_label.rs`), unit form: the
/// receipt's typed label is the REASON's labeled type. With the hand
/// removed the lane answers the plain declared base (String) and this
/// pin reddens — the end-to-end SECRET_LEAK would survive through the
/// lattice, so this unit pin is the only one that isolates the typed
/// hand.
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
    assert_eq!(
        with_secret,
        Some(secret_source_type()),
        "the typed lane must carry the REASON's secret type through the \
         receipt echo (№980) — got {with_secret:?}; the plain declared \
         base here masks a secret that rode args[1]"
    );
    let plain = label_of_expr(&call(lit("gdpr revoked")), &vars);
    assert_ne!(
        plain,
        Some(secret_source_type()),
        "a benign literal reason must NOT type as a secret — got {plain:?}"
    );
}

/// THE TABLE GROUNDING (the №951 lesson — a typo would disable the rule
/// silently): every echo name is a LIVE Role::Sink, keeps its receipt
/// membership (the echo REFINES the receipt claim, it does not replace
/// it), and the declared return is a String that can actually carry the
/// echo.
#[test]
fn n980_the_echo_table_is_grounded() {
    assert!(
        !RECEIPT_ECHO_ARGS.is_empty(),
        "the echo table is empty — the rule would be dead"
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
