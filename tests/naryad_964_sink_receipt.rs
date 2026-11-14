// ── №964 (the B7 line, the owner decision 13.11): a sink's RETURN is a
//    RECEIPT, not the payload ────────────────────────────────────────────
//
// THE FALSE POSITIVE, measured on this tree before the fix:
//
//   pattern P(d: String) -> String {
//     let r = quarantine_write("q", env("SP"))   // LEGAL — passes
//     let _ = print(r)                           // [SECRET_LEAK]
//     return "ok"
//   }
//
// The quarantine is THE sanctioned egress for a poisoned value (№325
// exempts it from the clearance gate), so the write itself passes. But
// the label of the payload came back OUT of the sink on its return value,
// so a program that quarantines a secret correctly and then logs the
// quarantine id was refused for logging the id. `w1_consent_revoke` is
// that program, and it is one of the three project examples the B7 line
// stumbled over.
//
// WHY A NAME TABLE AND NOT A RULE OVER RETURN TYPES. A blanket «a sink's
// scalar return is bottom» would be a LAUNDERING path for any sink that
// echoes its argument: `print(quarantine_write("q", secret))` would pass
// while printing the secret, because the quarantine call is legal and the
// echo would arrive at `print` unlabelled. Clearance for ONE channel is
// not clearance for another. So the claim is made per name, with the
// reason, in `builtins_classification::SINK_RECEIPT_NAMES`, and this file
// checks every name against the live registry — a typo would otherwise
// disable the rule silently (the №951 lesson).
//
// The by-type lane is the one claim that needs no per-name argument: a
// `Unit` or `Bool` return cannot carry a payload at all. `Float` and
// `String` are deliberately NOT in it.
#![allow(clippy::disallowed_methods)]

use metalogos::builtins_classification::{classify, is_sink_receipt, Role, SINK_RECEIPT_NAMES};

/// The refusal class of a program, or PASS / PARSE_ERROR.
fn verdict(src: &str) -> String {
    match metalogos::check_program(src) {
        Err(_) => "PARSE_ERROR".to_string(),
        Ok(r) if r.errors.is_empty() => "PASS".to_string(),
        Ok(r) => {
            let joined = r
                .errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            for line in joined.lines() {
                if let Some(s) = line.find('[') {
                    if let Some(e) = line[s + 1..].find(']') {
                        let id = &line[s + 1..s + 1 + e];
                        if !id.is_empty()
                            && id
                                .chars()
                                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                        {
                            return id.to_string();
                        }
                    }
                }
            }
            "UNCLASSED".to_string()
        }
    }
}

fn prog(body: &str) -> String {
    format!(
        "pattern P(d: String) -> String {{ {body} return \"ok\" }}\n\
         flow Main {{ input: String = \"x\" -> P -> output }}\n"
    )
}

/// THE FIX. The legal quarantine write passes, and so does using its
/// receipt. This is the test that was RED before the naryad.
#[test]
fn n964_the_receipt_of_a_legal_quarantine_write_is_usable() {
    let v = verdict(&prog(
        "let r = quarantine_write(\"q\", env(\"SP\")) let _ = print(r)",
    ));
    assert_eq!(
        v, "PASS",
        "a program that quarantines a secret CORRECTLY and logs the \
         quarantine id must not be refused for logging the id — the \
         receipt carries no payload (№964)"
    );
}

/// THE MUTATION TARGET / the soundness half. The receipt rule must NOT
/// excuse the sink CALL: writing a secret to an ordinary file is still a
/// leak, receipt or no receipt.
#[test]
fn n964_the_receipt_rule_does_not_excuse_the_call() {
    for body in [
        "let r = write_file(\"f.txt\", env(\"SP\")) let _ = print(r)",
        "let _ = write_file(\"f.txt\", env(\"SP\"))",
        "let r = write_file(\"f.txt\", env(\"SP\"))",
    ] {
        let v = verdict(&prog(body));
        assert_ne!(
            v, "PASS",
            "writing a secret to an ordinary file is a leak at the CALL; \
             the receipt rule is about the RETURN value only. Body: {body}"
        );
    }
}

/// The receipt is bottom, not «whatever the payload was»: a receipt
/// reaching a DIFFERENT sink is fine, because it is an id. The paired
/// negative is the test above — the call itself still answers for the
/// payload.
#[test]
fn n964_a_receipt_may_travel_to_another_sink() {
    let v = verdict(&prog(
        "let r = quarantine_write(\"q\", env(\"SP\")) let _ = write_file(\"audit.txt\", r)",
    ));
    assert_eq!(
        v, "PASS",
        "the quarantine id is an id — recording it in an audit file is \
         the intended use of a receipt"
    );
}

/// Every name in the table is a LIVE `Role::Sink` in the registry. A
/// renamed or reclassified builtin must break this test rather than
/// silently keep an exemption it no longer deserves.
#[test]
fn n964_every_receipt_name_is_a_live_sink() {
    assert!(
        !SINK_RECEIPT_NAMES.is_empty(),
        "the receipt table is empty — the rule would be dead"
    );
    for name in SINK_RECEIPT_NAMES {
        let c = classify(name).unwrap_or_else(|| {
            panic!(
                "`{name}` stands in SINK_RECEIPT_NAMES but is not in the \
                 classification at all — a typo disables the rule silently"
            )
        });
        assert!(
            matches!(c.role, Role::Sink),
            "`{name}` is in SINK_RECEIPT_NAMES but is classified {:?}, not \
             Sink — the table claims a fact about SINKS",
            c.role
        );
        assert!(
            metalogos::builtins::BUILTIN_REGISTRY
                .iter()
                .any(|s| s.name == *name),
            "`{name}` is in SINK_RECEIPT_NAMES but not in the registry"
        );
        assert!(
            is_sink_receipt(name),
            "`{name}` is in the table but the predicate says it is not a \
             receipt — the two must not disagree"
        );
    }
}

/// The DELIVERY sinks are deliberately NOT receipts: their returns may
/// carry the request, nothing in the registry says otherwise, and the
/// conservative join costs those programs nothing. This test is the
/// guard against someone widening the table into the egress family —
/// the exact shape of the №951 B6 lesson.
#[test]
fn n964_the_delivery_sinks_are_not_receipts() {
    for name in [
        "print",
        "http_post",
        "respond",
        "respond_html",
        "smtp_send",
        "exec",
        "git_push",
        "tts_send",
        "human_respond",
    ] {
        assert!(
            !SINK_RECEIPT_NAMES.contains(&name),
            "`{name}` is a DELIVERY sink — it must never be a receipt"
        );
        assert!(
            !is_sink_receipt(name),
            "the predicate calls `{name}` a receipt — a delivery sink's \
             return is not a handle, and treating it as one is a \
             laundering path"
        );
    }
}

/// The by-type lane: `Unit`/`Bool` returns are receipts without needing a
/// name, because such a value cannot carry a payload. `Float` and
/// `String` must NOT ride this lane — they can echo.
#[test]
fn n964_the_by_type_lane_covers_unit_and_bool_only() {
    let mut by_type = 0;
    for spec in metalogos::builtins::BUILTIN_REGISTRY.iter() {
        let Some(c) = classify(spec.name) else {
            continue;
        };
        if !matches!(c.role, Role::Sink) {
            continue;
        }
        let named = SINK_RECEIPT_NAMES.contains(&spec.name);
        let receipt = is_sink_receipt(spec.name);
        match spec.return_type_raw {
            "Unit" | "Bool" => {
                assert!(
                    receipt,
                    "`{}` returns {} — it cannot carry a payload, so it is \
                     a receipt by type",
                    spec.name, spec.return_type_raw
                );
                if !named {
                    by_type += 1;
                }
            }
            "String" | "Float" | "Int" => assert_eq!(
                receipt, named,
                "`{}` returns {} — it may ECHO its argument, so it is a \
                 receipt ONLY if the table names it with a reason",
                spec.name, spec.return_type_raw
            ),
            _ => assert_eq!(
                receipt, named,
                "`{}` has an unknown/struct return ({}) — fail-closed: the \
                 argument join stands unless the table names it",
                spec.name, spec.return_type_raw
            ),
        }
    }
    assert!(
        by_type > 0,
        "the by-type lane covers nothing — if every Unit/Bool sink is also \
         named, the lane is dead code and should be removed rather than \
         left as decoration"
    );
}

/// A typed-struct return keeps its DECLARED field labels: the registry
/// says what `todo_add`'s struct carries, and the receipt rule must not
/// overwrite that with bottom. (The private-field bridge runs first in
/// both lanes; this test pins the ordering.)
#[test]
fn n964_a_typed_struct_return_keeps_its_declared_field_labels() {
    let spec = metalogos::builtins::BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == "todo_add")
        .expect("todo_add is registered");
    assert!(
        spec.return_type_raw.starts_with("Struct<"),
        "todo_add's declared return is expected to be a typed struct; got {}",
        spec.return_type_raw
    );
    assert!(
        !is_sink_receipt("todo_add"),
        "a typed-struct return declares its own field labels — the receipt \
         rule must leave it to the private-field bridge, not flatten it"
    );
}

/// Both label lanes agree. The lattice (`semantic`) and the typed secret
/// lane (`secret_label`) consult the SAME predicate; this test is the
/// end-to-end witness that they do, through a program each lane answers.
#[test]
fn n964_both_label_lanes_honour_the_receipt() {
    // The secret lane's class is SECRET_LEAK; the lattice's is
    // PII_EGRESS_*/TAINT_PERSISTENCE. A receipt must be clean on BOTH, so
    // neither class may appear.
    //
    // BOTH calls here are LEGAL, which is the whole point: `quarantine_write`
    // is the sanctioned egress for a RAW secret (№325), and `kv_set` is
    // legal only for a REDACTED value (ADR-0136 redact-before-persist —
    // `kv_set("k", env("SP"))` refuses at the call with
    // [TAINT_PERSISTENCE], receipt or no receipt, and the first draft of
    // this test wrongly used that form). The receipt rule changes which
    // RETURN values carry a label; it never changes which calls are legal.
    let v = verdict(&prog(
        "let r = quarantine_write(\"q\", env(\"SP\")) let _ = print(r) \
         let s = kv_set(\"k\", redact(env(\"SP\"), \"all\")) let _ = print(s)",
    ));
    assert_eq!(
        v, "PASS",
        "the receipt of a legal quarantine write and of a kv_set must be \
         clean on both label lanes — a disagreement between the lanes is \
         the class that is never pinnable (№476)"
    );
}
