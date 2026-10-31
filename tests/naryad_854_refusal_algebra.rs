// ── Naryad №854 (wave 74 — W-C, 8F-1/C6, ADR-0204): the refusal algebra
// lands and becomes the ONLY construction path of the runtime
// `DenyEvent` — with the surface and the Ledger records BYTE-IDENTICAL
// (the №844 1:1 migration discipline; the ledger-golden and conformance
// suites must pass with ZERO golden edits).
//
// What this file pins:
//   1. the algebra is total over the frozen 14-word vocabulary and
//      closed outside it (ADR-0204 §6 — no new codes);
//   2. the chain-lattice laws (join/meet) hold over the FULL 14×14
//      table;
//   3. verdicts carry the frozen stable refusal code (the code IS the
//      reason word for gate refusals);
//   4. the migrated `make_event` produces the byte-identical 7-field
//      `DenyEvent` for every vocabulary word — the parity contract;
//   5. a non-vocabulary word keeps the legacy passthrough (unreachable
//      from the gate dictionary — every construction site passes a
//      `DENY_REASONS` word), so the migration invents no new failure
//      mode.
#![allow(clippy::disallowed_methods)]

use metalogos::deny;
use metalogos::interpreter::values::Value;
use metalogos::refusal_algebra::{join, meet, Reason, Verdict, DENY_REASONS};

fn legacy_event(
    reason: &str,
    sink: &str,
    class: &str,
    argument: &str,
    label: &str,
    line: f64,
    human: &str,
) -> Value {
    // The PRE-№854 construction, verbatim (the parity oracle): the same
    // 7 fields, the same struct type name.
    let mut fields = std::collections::HashMap::new();
    fields.insert("reason".to_string(), Value::String(reason.to_string()));
    fields.insert("sink".to_string(), Value::String(sink.to_string()));
    fields.insert("class".to_string(), Value::String(class.to_string()));
    fields.insert("argument".to_string(), Value::String(argument.to_string()));
    fields.insert("label".to_string(), Value::String(label.to_string()));
    fields.insert("line".to_string(), Value::Float(line));
    fields.insert("human".to_string(), Value::String(human.to_string()));
    Value::Struct {
        type_name: "DenyEvent".to_string(),
        fields,
    }
}

fn assert_same_event(a: &Value, b: &Value) {
    match (a, b) {
        (
            Value::Struct {
                type_name: tn1,
                fields: f1,
            },
            Value::Struct {
                type_name: tn2,
                fields: f2,
            },
        ) => {
            assert_eq!(tn1, tn2);
            assert_eq!(f1.len(), f2.len());
            for (k, v) in f1 {
                // `Value` carries no PartialEq — the field comparison goes
                // through the derived Debug (deterministic per value).
                assert_eq!(
                    format!("{:?}", f2.get(k)),
                    format!("{:?}", Some(v)),
                    "field {k} drifted"
                );
            }
        }
        _ => panic!("both sides must be Structs"),
    }
}

#[test]
fn the_frozen_vocabulary_is_the_lattice_domain() {
    let mut ranks = std::collections::HashSet::new();
    for w in DENY_REASONS {
        let r = Reason::parse(w).unwrap_or_else(|| panic!("the vocabulary word must parse: {w}"));
        assert_eq!(r.word(), w);
        assert!(ranks.insert(r.rank()), "duplicate rank for {w}");
    }
    assert_eq!(ranks.len(), 14);
    assert!(Reason::parse("NOT_A_REASON").is_none());
    assert!(Reason::parse("").is_none());
    assert!(Reason::parse("secret_leak").is_none());
}

#[test]
fn the_chain_lattice_laws_hold_over_the_full_table() {
    let rs: Vec<Reason> = DENY_REASONS
        .iter()
        .map(|w| Reason::parse(w).unwrap())
        .collect();
    for a in &rs {
        for b in &rs {
            let j = join(*a, *b);
            let m = meet(*a, *b);
            assert_eq!(j, join(*b, *a), "join commutative");
            assert_eq!(m, meet(*b, *a), "meet commutative");
            assert!(j.rank() <= m.rank(), "join dominates meet");
            assert_eq!(join(*a, m), *a, "absorption (join)");
            assert_eq!(meet(*a, j), *a, "absorption (meet)");
            assert_eq!(join(*a, *a), *a, "idempotent join");
            assert_eq!(meet(*a, *a), *a, "idempotent meet");
        }
    }
    for a in &rs {
        for b in &rs {
            for c in &rs {
                assert_eq!(join(join(*a, *b), *c), join(*a, join(*b, *c)));
                assert_eq!(meet(meet(*a, *b), *c), meet(*a, meet(*b, *c)));
            }
        }
    }
    // The severity definition (ADR-0204 §2): the core class dominates
    // the legacy one.
    let core = Reason::parse("IRREVERSIBLE_NO_GRANT").unwrap();
    let legacy = Reason::parse("SECRET_LEAK").unwrap();
    assert_eq!(join(core, legacy), core);
    assert_eq!(meet(core, legacy), legacy);
}

#[test]
fn verdicts_carry_the_frozen_code_and_the_cause() {
    for w in DENY_REASONS {
        let v =
            Verdict::deny(w).unwrap_or_else(|| panic!("the vocabulary word builds a verdict: {w}"));
        assert!(v.is_deny());
        assert_eq!(v.code(), Some(w));
    }
    assert!(Verdict::deny("NOT_A_REASON").is_none());
}

#[test]
fn allow_is_the_bottom_element_of_the_verdict_lattice() {
    for w in DENY_REASONS {
        let d = Verdict::deny(w).unwrap();
        assert_eq!(Verdict::Allow.join(d.clone()), d);
        assert_eq!(d.clone().join(Verdict::Allow), d);
        assert_eq!(Verdict::Allow.meet(d.clone()), Verdict::Allow);
        assert_eq!(d.clone().meet(Verdict::Allow), Verdict::Allow);
    }
}

#[test]
fn make_event_is_byte_identical_for_every_vocabulary_word() {
    for w in DENY_REASONS {
        let ev = deny::make_event(
            w,
            "db_execute_with_grant",
            "db",
            "sql",
            "bottom",
            7.0,
            "GRANT_EXHAUSTED: grant g1 is exhausted",
        );
        let expected = legacy_event(
            w,
            "db_execute_with_grant",
            "db",
            "sql",
            "bottom",
            7.0,
            "GRANT_EXHAUSTED: grant g1 is exhausted",
        );
        assert_same_event(&ev, &expected);
    }
}

#[test]
fn make_event_parity_holds_for_the_tw_and_vm_argument_shapes() {
    // The TW hook shape (src/interpreter/hooks.rs) and the VM twin
    // (src/vm.rs) both funnel through make_event — the exact argument
    // bundles the executors pass today.
    let tw = deny::make_event(
        "IRREVERSIBLE_NO_GRANT",
        "http_post",
        "network",
        "call",
        "bottom",
        0.0,
        "the sharp carrier refuses without a covering capability grant",
    );
    let expected = legacy_event(
        "IRREVERSIBLE_NO_GRANT",
        "http_post",
        "network",
        "call",
        "bottom",
        0.0,
        "the sharp carrier refuses without a covering capability grant",
    );
    assert_same_event(&tw, &expected);
}

#[test]
fn a_non_vocabulary_word_keeps_the_legacy_passthrough() {
    // Unreachable from the gate dictionary (every construction site
    // passes a DENY_REASONS word) — but the 1:1 migration must not
    // invent a new failure mode even here: the event renders the raw
    // word, exactly as pre-№854.
    let ev = deny::make_event("NOT_IN_THE_TABLE", "sink", "db", "arg", "bottom", 3.0, "h");
    let expected = legacy_event("NOT_IN_THE_TABLE", "sink", "db", "arg", "bottom", 3.0, "h");
    assert_same_event(&ev, &expected);
}

#[test]
fn the_ledger_key_format_is_untouched_by_the_migration() {
    // The Ledger record key is `deny.{reason}` (ADR-0167 §3.4) built
    // from the SAME reason string the executors pass — the migration
    // must not perturb it. This pins the format contract the
    // ledger-golden suite (№847) exercises end-to-end.
    for w in DENY_REASONS {
        assert_eq!(
            format!("deny.{}", w),
            format!("deny.{}", Reason::parse(w).unwrap().word())
        );
    }
}
