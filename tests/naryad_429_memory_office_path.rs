//! Naryad №429 (P1, feature/memory): the OFFICE-PATH integration contract
//! for typed Memory<K> + cascade + ledger (audit 22.09 P1-1).
//!
//! The office scenario end to end, THROUGH THE LANGUAGE SURFACE:
//! session_login → consent_grant → memory_open<private> → put (with
//! derived-from provenance) → audited read (the private payload leaves
//! the store only through redact — the №326 sanctioned downward move) →
//! cascade preview → the grant spend on the irreversible cascade
//! (ADR-0155 §3.5) → derived gone, independent intact → every step in
//! the Action Ledger (`memory.put` / `memory.read` / `memory.forget_cascade`).
//!
//! Coverage the audit named as missing: cascade ×0 and ledger ×0 in
//! tests/session_memory_contract.rs — closed here; the static-boundary
//! row lands in docs/limitations.md.

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
use metalogos::ledger::all_records;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// The consent ledger AND the action ledger are process-global — the
/// office-path tests serialize on one lock (the №251 flake family).
fn test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn call_builtin(name: &str, args: &[Value]) -> Result<Value, String> {
    let spec = BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("builtin {name} not in BUILTIN_REGISTRY"));
    let handler = spec.handler.expect("builtin has handler");
    handler(args)
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn count_kind(kinds: &[String], kind: &str) -> usize {
    kinds.iter().filter(|k| k.as_str() == kind).count()
}

fn ledger_kinds() -> Vec<String> {
    all_records()
        .expect("action ledger must be readable")
        .into_iter()
        .map(|r| r.action)
        .collect()
}

/// The full office path: private memory + consent + derived-from + grant
/// cascade + ledger, asserted end to end in ONE scenario (the audit's
/// «office-path: сессия → private-память → consent → каскад с грантом →
/// ledger-след»).
#[test]
fn n429_office_path_end_to_end() {
    let _g = test_lock();

    // 1. The session (№348 surface) — the office actor logs in.
    call_builtin(
        "session_login",
        &[s("office-user-429"), s("pw-not-checked-by-design")],
    )
    .expect("session_login");

    // 2. Consent first: a private container refuses without it (№350 M5).
    let err = call_builtin("memory_open", &[s("office-user-429"), s("private")])
        .expect_err("private open without consent must refuse");
    assert!(
        err.contains("MEMORY_CONSENT_REQUIRED"),
        "the private-open refusal must name the consent gate; got: {err}"
    );
    call_builtin(
        "consent_grant",
        &[s("office-user-429"), s("memory:office-user-429"), s("office-user-429")],
    )
    .expect("consent_grant arms the private contour");

    // 3. The private container opens; entries carry derived-from provenance.
    let mem = call_builtin("memory_open", &[s("office-user-429"), s("private")])
        .expect("private open with consent");
    call_builtin(
        "memory_put",
        &[mem.clone(), s("source1"), s("raw one"), Value::List(vec![])],
    )
    .expect("put source1");
    call_builtin(
        "memory_put",
        &[mem.clone(), s("source2"), s("raw two"), Value::List(vec![])],
    )
    .expect("put source2");
    call_builtin(
        "memory_put",
        &[
            mem.clone(),
            s("summary"),
            s("the summary"),
            Value::List(vec![s("source1"), s("source2")]),
        ],
    )
    .expect("put summary (derived from both sources)");
    call_builtin(
        "memory_put",
        &[
            mem.clone(),
            s("standalone"),
            s("an unrelated fact"),
            Value::List(vec![]),
        ],
    )
    .expect("put standalone");

    // 4. The audited read: private content leaves through redact only.
    let raw = call_builtin("memory_read", &[mem.clone(), s("summary")])
        .expect("the private read itself is audited, not refused");
    match &raw {
        Value::Secret(_) | Value::Encrypted(_) => { /* the lattice seals it */ }
        other => panic!(
            "the private read must stay sealed (Secret/Encrypted), got {}",
            other.type_name()
        ),
    }
    let redacted = call_builtin("redact", &[raw.clone(), s("hash_only")])
        .expect("redact is the sanctioned downward move");
    match redacted {
        Value::String(h) => assert!(!h.is_empty(), "redact yields a digest"),
        other => panic!("redact must yield a String, got {}", other.type_name()),
    }

    // 5. The preview (the №280 dry-run discipline) then the GRANT spend:
    //    the cascade is irreversible — it costs a Once grant (ADR-0155).
    let plan = call_builtin("memory_cascade_preview", &[mem.clone(), s("source1")])
        .expect("cascade_preview");
    let reach = match &plan {
        Value::Struct { fields, .. } => match fields.get("closure") {
            Some(Value::List(items)) => items.len(),
            other => panic!("closure must be a List, got {:?}", other.is_some()),
        },
        other => panic!("preview must be a Struct, got {}", other.type_name()),
    };
    assert_eq!(reach, 2, "source1 reaches summary -> (its own) closure: source1 + summary");

    let grant = call_builtin(
        "grant_issue",
        &[s("memory:forget:*"), Value::Float(60.0), s("once")],
    )
    .expect("grant_issue");

    // 6. WITHOUT the grant the cascade refuses typed (the negative).
    let err = call_builtin(
        "memory_forget_cascade",
        &[mem.clone(), s("source1"), s("not-a-grant")],
    )
    .expect_err("cascade without a Grant refuses");
    assert!(
        err.contains("GRANT_MISSING"),
        "the no-grant refusal must be typed GRANT_MISSING; got: {err}"
    );

    // 7. The granted cascade: derived gone, independent intact.
    let before_put = count_kind(&ledger_kinds(), "memory.put");
    call_builtin("memory_forget_cascade", &[mem.clone(), s("source1"), grant])
        .expect("the granted cascade succeeds");
    assert_eq!(
        count_kind(&ledger_kinds(), "memory.put"),
        before_put,
        "sanity: the cascade records its own kind, not memory.put"
    );
    let keys = call_builtin("memory_keys", &[mem]).expect("memory_keys");
    let mut names: Vec<String> = match &keys {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::String(k) => k.clone(),
                other => panic!("key must be String, got {}", other.type_name()),
            })
            .collect(),
        other => panic!("keys must be a List, got {}", other.type_name()),
    };
    names.sort();
    assert_eq!(
        names,
        vec!["source2".to_string(), "standalone".to_string()],
        "the cascade deleted source1 AND its transitive derivative (summary); independent entries survive"
    );
}

/// The ledger completeness of the office path: put, read, preview and
/// cascade each leave exactly their record family (the audit's «ledger
/// фиксирует cascade forget»).
#[test]
fn n429_ledger_records_every_office_step() {
    let _g = test_lock();
    call_builtin(
        "consent_grant",
        &[s("ledger-user-429"), s("memory:ledger-user-429"), s("ledger-user-429")],
    )
    .expect("consent");
    let mem = call_builtin("memory_open", &[s("ledger-user-429"), s("private")])
        .expect("open");
    let before = ledger_kinds();
    call_builtin(
        "memory_put",
        &[mem.clone(), s("root"), s("value"), Value::List(vec![])],
    )
    .expect("put");
    call_builtin("memory_read", &[mem.clone(), s("root")]).expect("read");
    call_builtin("memory_cascade_preview", &[mem.clone(), s("root")])
        .expect("preview");
    let grant = call_builtin(
        "grant_issue",
        &[s("memory:forget:*"), Value::Float(60.0), s("once")],
    )
    .expect("grant");
    call_builtin("memory_forget_cascade", &[mem, s("root"), grant]).expect("cascade");

    let after = ledger_kinds();
    assert_eq!(
        count_kind(&after, "memory.put"),
        count_kind(&before, "memory.put") + 1,
        "memory.put recorded"
    );
    assert_eq!(
        count_kind(&after, "memory.read"),
        count_kind(&before, "memory.read") + 1,
        "memory.read recorded (the sink audit)"
    );
    assert_eq!(
        count_kind(&after, "memory.cascade_preview"),
        count_kind(&before, "memory.cascade_preview") + 1,
        "memory.cascade_preview recorded"
    );
    assert_eq!(
        count_kind(&after, "memory.forget_cascade"),
        count_kind(&before, "memory.forget_cascade") + 1,
        "memory.forget_cascade recorded — the audit's P1-1 (в) criterion"
    );
}

/// The static-boundary companion: the public container needs NO consent
/// (the office can keep public memory without the consent contour) —
/// the boundary №429 documents in limitations.md stays honest.
#[test]
fn n429_public_container_needs_no_consent() {
    let _g = test_lock();
    let mem = call_builtin("memory_open", &[s("public-user-429"), s("public")])
        .expect("public open needs no consent");
    call_builtin("memory_put", &[mem, s("k"), s("v"), Value::List(vec![])])
        .expect("public put");
}

/// №16.0-D: no stubs in this test file (markers assembled from parts).
#[test]
fn n429_no_stubs() {
    let src = std::fs::read_to_string(file!()).unwrap_or_default();
    let bang = String::from("!");
    for m in ["todo", "unimplemented"] {
        let marker = format!("{}{}", m, bang);
        assert!(!src.contains(&marker), "stub marker {} found", marker);
    }
    let skeleton = ["SKELE", "TON"].concat();
    assert!(!src.contains(&skeleton), "stub marker (assembled) found");
}
