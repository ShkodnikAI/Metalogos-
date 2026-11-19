// ── №977 (issue #1986, the consolidated audit 18.11 — finding B): the
//    three echo names OUT of the receipt table ─────────────────────────
//
// THE FACT, verified in the bounds of each handler body (never a
// fixed-size window): `mem_set` returns args[1] — THE VALUE stored,
// `session_set` returns args[2] — THE VALUE stored, `mem_delete`
// returns the value removed FROM THE STORE (src/builtins/memory.rs).
// The receipt table claimed «the key» for all three; the claim was
// false, and the receipt rule became a laundering path — exactly the
// shape the sink_receipt module warning describes.
//
// THE PROBE CLASS: a NON-MASKABLE value. `query_param` is untrusted
// (UserInput taint) but NOT a secret, so neither the secret lane nor
// the persistence rule hides the result — the verdict difference is
// attributable to the label alone:
//
//   control        query_param(q) → respond   = HTML_INJECTION  (always)
//   session_set(s,k,query_param(q)) → respond = HTML_INJECTION was GONE
//   mem_set(k,query_param(q))       → respond = HTML_INJECTION was GONE
//   mem_delete(k)                   → respond = PASS (no refusal at all)
//
// THE CONTROL SITS IN THIS FILE, next to the pins: without it the pins
// cannot prove that the missing class is the defect and not the
// baseline. THE MUTATION TARGET: re-add any of the three names to
// `SINK_RECEIPT_NAMES` (or widen `is_sink_receipt` to cover them) and
// every pin in this file reddens — the receipt rule would clean the
// echoes again.
#![allow(clippy::disallowed_methods)]

use metalogos::builtins_classification::{classify, Role};
use metalogos::sink_receipt::{is_sink_receipt, SINK_RECEIPT_NAMES};

/// Every refusal class of a program (PARSE_ERROR / PASS yield empty).
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

/// THE CONTROL — the baseline that proves the class exists on this
/// tree and that the harness sees it. Without this the three pins
/// below are uninterpretable.
#[test]
fn n977_the_control_refuses_html_injection() {
    let v = classes(&prog("let v = query_param(\"q\") let _ = respond(v)"));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "the bare control `query_param → respond` must carry \
         HTML_INJECTION — got {v:?}; if this is red the probe harness \
         (or the respond gate) changed, and the three pins below mean \
         nothing until it is green"
    );
}

/// session_set's return is args[2] — the VALUE. The echo must not
/// launder the label: respond must refuse again, exactly like the
/// control.
#[test]
fn n977_session_set_echo_no_longer_launder() {
    let v = classes(&prog(
        "let v = session_set(\"s\", \"k\", query_param(\"q\")) let _ = respond(v)",
    ));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "session_set returns the stored VALUE (memory.rs, args[2]) — \
         the return must carry the argument join again, so respond \
         refuses with HTML_INJECTION like the control — got {v:?}"
    );
}

/// mem_set's return is args[1] — the VALUE. Same laundering shape as
/// session_set; the call itself may additionally refuse on
/// persistence, which is fine — the pin is on HTML_INJECTION being
/// PRESENT, not on the single-class verdict.
#[test]
fn n977_mem_set_echo_no_longer_launder() {
    let v = classes(&prog(
        "let v = mem_set(\"k\", query_param(\"q\")) let _ = respond(v)",
    ));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "mem_set returns the stored VALUE (memory.rs, args[1]) — the \
         return must carry the argument join again, so respond refuses \
         with HTML_INJECTION like the control — got {v:?}"
    );
}

/// mem_delete's return is the value removed FROM THE STORE — store
/// content is statically unknowable, so the only sound approximation
/// is the argument join. On the pre-№977 tree this path did not
/// refuse AT ALL (PASS): the sharpest form of the finding. With `k`
/// untrusted, respond must refuse again.
#[test]
fn n977_mem_delete_echo_no_longer_launder() {
    let src = prog("let k = query_param(\"q\") let v = mem_delete(k) let _ = respond(v)");
    let v = classes(&src);
    assert!(
        !v.is_empty(),
        "mem_delete(k) with an untrusted key must refuse — on the \
         pre-№977 tree this was PASS, the receipt rule cleared the \
         removed store value — got empty (PASS)"
    );
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "mem_delete's echo must carry the join of its arguments into \
         respond — HTML_INJECTION expected — got {v:?}"
    );
}

/// kv_set KEEPS today's (correct) behavior: it returns Unit, the
/// receipt claim holds, so the respond leg stays clean — the verdict
/// comes from the CALL (persistence), never from the return.
#[test]
fn n977_kv_set_keeps_its_correct_verdict() {
    let v = classes(&prog(
        "let v = kv_set(\"k\", query_param(\"q\")) let _ = respond(v)",
    ));
    assert!(
        !v.is_empty(),
        "kv_set of untrusted data must refuse at the CALL (persistence) \
         — the audit measured TAINT_PERSISTENCE on this shape"
    );
    assert!(
        v.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the persistence class on the kv_set call must stay — got {v:?}"
    );
    assert!(
        !v.iter().any(|c| c == "HTML_INJECTION"),
        "kv_set returns Unit — a receipt by type; the respond leg must \
         stay clean (the audit measured this as correct behavior) — got \
         {v:?}"
    );
}

/// The table mutation guard, structural form: the three echo names are
/// OUT, and they are still live `Role::Sink` entries — the removal is
/// not a reclassification, it is the receipt claim being retracted.
#[test]
fn n977_the_three_echo_names_are_out_of_the_table() {
    for name in ["mem_set", "session_set", "mem_delete"] {
        assert!(
            !SINK_RECEIPT_NAMES.contains(&name),
            "`{name}` returned the VALUE (or the removed store value) — \
             it must never stand in SINK_RECEIPT_NAMES again; re-adding \
             it re-opens the laundering path the pins in this file guard"
        );
        assert!(
            !is_sink_receipt(name),
            "the predicate still calls `{name}` a receipt — the table and \
             the predicate must not disagree"
        );
        let c = classify(name).expect("the echo builtins stay classified");
        assert!(
            matches!(c.role, Role::Sink),
            "`{name}` is still a sink — №977 retracts the receipt claim, \
             it does not reclassify the builtin"
        );
    }
}

/// The rest of the table stands: №977 removes three names, not the
/// rule. Over-removal would re-break the motivating false positive
/// (the legal quarantine receipt), so the surviving names are pinned.
#[test]
fn n977_the_rest_of_the_table_stands() {
    for name in SINK_RECEIPT_NAMES {
        assert!(
            is_sink_receipt(name),
            "`{name}` stayed in the table but the predicate answers false \
             — №977 must not hollow out the table it did not touch"
        );
    }
    for name in [
        "quarantine_write",
        "write_file",
        "append_file",
        "delete_file",
        "db_insert",
        "approval_revoke",
        "kv_set",
        "kv_delete",
        "memorize",
        "remind",
    ] {
        assert!(
            is_sink_receipt(name),
            "`{name}` is a verified receipt (№977 re-read every handler \
             body) — its claim must survive the three-name removal"
        );
    }
}

/// Both label lanes agree on the probes: the lattice refuses
/// (HTML_INJECTION), and the typed secret lane does NOT call the
/// untrusted-but-not-secret query_param value a secret — the lanes
/// cannot disagree about what an echo is now that the names left the
/// table: both fall back to the argument join.
#[test]
fn n977_the_two_label_lanes_agree_on_the_probes() {
    for (label, body) in [
        ("control", "let v = query_param(\"q\") let _ = respond(v)"),
        (
            "session_set",
            "let v = session_set(\"s\", \"k\", query_param(\"q\")) let _ = respond(v)",
        ),
        (
            "mem_set",
            "let v = mem_set(\"k\", query_param(\"q\")) let _ = respond(v)",
        ),
        (
            "mem_delete",
            "let k = query_param(\"q\") let v = mem_delete(k) let _ = respond(v)",
        ),
    ] {
        let v = classes(&prog(body));
        assert!(
            v.iter().any(|c| c == "HTML_INJECTION"),
            "{label}: the integrity lane must refuse — got {v:?}"
        );
        assert!(
            !v.iter().any(|c| c == "SECRET_LEAK"),
            "{label}: query_param is untrusted but NOT a secret — the \
             typed secret lane must not raise it; a SECRET_LEAK here is \
             a lane disagreement (№476), not a fix — got {v:?}"
        );
    }
}
