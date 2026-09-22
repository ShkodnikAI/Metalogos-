// ── Naryad #351 (P1, feature/memory): the derived-from graph and the
//    cascading forget — contracts (ADR-0173) ───────────────────────────
//
//   C1  the cascade closure: forget_cascade(root) deletes the root AND
//       every transitive derived-from descendant; independent entries
//       survive with their provenance intact (P1/P2/P3).
//   C2  the preview (the №280 dry-run discipline): {closure, blocked_by}
//       matches the subsequent forget exactly; no state change, no grant
//       consumption on preview.
//   C3  the retained VETO: a retained node inside the closure refuses
//       the WHOLE forget (MEMORY_RETAIN_PROTECTED), nothing is deleted,
//       the grant is NOT consumed (the grant stays usable afterwards).
//   C4  retain/release are the cascading pin/unpin of the descendant
//       closure; pins survive value overwrites; release is idempotent.
//   C5  the grant path (ADR-0155 §3.5): GRANT_MISSING without a grant,
//       GRANT_SCOPE_MISMATCH on a foreign scope, Once linearity
//       (GRANT_REUSED on the second use), N-exhaustion (GRANT_EXHAUSTED),
//       revocation (GRANT_REVOKED), expiry (GRANT_EXPIRED).
//   C6  the post-success journal: `irreversible.memory_forget` records
//       the grant id + a digest; the values NEVER enter the journal;
//       consumption and journal are side effects of SUCCESS only.

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::grants::{self, GrantClass};
use metalogos::interpreter::Value;
use metalogos::ledger::all_records;
use metalogos::memory_typed;

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

fn list(vs: &[&str]) -> Value {
    Value::List(vs.iter().map(|v| s(v)).collect())
}

fn unique(tag: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}-{}", tag, std::process::id(), nanos)
}

fn open_public(subject: &str) -> Value {
    call_builtin("memory_open", &[s(subject), s("public")]).expect("public open needs no consent")
}

fn handle_id(handle: &Value) -> String {
    match handle {
        Value::Memory(map) => map.get("id").cloned().expect("handle has id"),
        other => panic!("expected Memory handle, got {}", other.type_name()),
    }
}

fn put(handle: &Value, key: &str, parents: &[&str]) {
    let mut args = vec![handle.clone(), s(key), s(&format!("value-of-{}", key))];
    if !parents.is_empty() {
        args.push(list(parents));
    }
    call_builtin("memory_put", &args).unwrap_or_else(|e| panic!("put {key}: {e}"));
}

fn keys(handle: &Value) -> Vec<String> {
    match call_builtin("memory_keys", std::slice::from_ref(handle)).expect("keys") {
        Value::List(items) => items
            .into_iter()
            .map(|v| match v {
                Value::String(x) => x,
                other => panic!("key not a string: {}", other.type_name()),
            })
            .collect(),
        other => panic!("keys not a list: {}", other.type_name()),
    }
}

fn struct_field(v: &Value, name: &str) -> Value {
    match v {
        Value::Struct { fields, .. } => fields
            .iter()
            .find(|(k, _)| k.as_str() == name)
            .map(|(_, val)| val.clone())
            .unwrap_or_else(|| panic!("struct has no field {name}")),
        other => panic!("expected Struct, got {}", other.type_name()),
    }
}

fn str_field(v: &Value, name: &str) -> String {
    match struct_field(v, name) {
        Value::String(x) => x,
        other => panic!("field {name} not a string: {}", other.type_name()),
    }
}

fn str_list(v: &Value) -> Vec<String> {
    match v {
        Value::List(items) => items
            .iter()
            .map(|x| match x {
                Value::String(t) => t.clone(),
                other => panic!("list element not a string: {}", other.type_name()),
            })
            .collect(),
        other => panic!("expected List, got {}", other.type_name()),
    }
}

/// The office shape under test (the ADR §3.3 example, corrected):
///   sources s1, s2 (independent roots)
///   mid  ← derived from [s1, s2]
///   leaf ← derived from [mid]
///   side ← derived from [s1]      (a second branch of s1)
/// forget_cascade(s1) must delete {s1, mid, leaf, side} and leave s2;
/// forget_cascade(s2) must delete {s2, mid, leaf} and leave s1, side.
fn build_office_graph(tag: &str) -> (Value, String, String, String, String, String) {
    let subject = unique(tag);
    let handle = open_public(&subject);
    let h = handle_id(&handle);
    put(&handle, "s1", &[]);
    put(&handle, "s2", &[]);
    put(&handle, "mid", &["s1", "s2"]);
    put(&handle, "leaf", &["mid"]);
    put(&handle, "side", &["s1"]);
    (
        handle,
        h,
        "s1".into(),
        "s2".into(),
        "mid".into(),
        "leaf".into(),
    )
}

fn issue_grant(scope: &str, ttl: f64, class: &str, uses: Option<f64>) -> Value {
    let mut args = vec![s(scope), Value::Float(ttl), s(class)];
    if let Some(u) = uses {
        args.push(Value::Float(u));
    }
    call_builtin("grant_issue", &args).expect("grant_issue succeeds")
}

// ── C1: the cascade closure deletes exactly the transitive derivatives ─

#[test]
fn cascade_forget_deletes_full_closure_and_nothing_else() {
    let (handle, _h, s1, s2, _mid, _leaf) = build_office_graph("n351-c1");
    let grant = issue_grant("memory:forget:*", 3600.0, "unlimited", None);
    let out = call_builtin("memory_forget_cascade", &[handle.clone(), s(&s1), grant])
        .expect("unblocked cascade forget succeeds");
    assert_eq!(str_field(&out, "root"), s1);
    let mut deleted = str_list(&struct_field(&out, "deleted"));
    deleted.sort();
    assert_eq!(
        deleted,
        vec!["leaf", "mid", "s1", "side"],
        "P2: the full closure dies"
    );
    let rest = keys(&handle);
    assert_eq!(rest, vec![s2.clone()], "P3: the independent s2 survives");
    // P1: the survivor's provenance is intact — s2 has no parents, and
    // its own readable value proves the entry chain was not touched.
    match call_builtin("memory_read", &[handle.clone(), s(&s2)]).expect("survivor readable") {
        Value::String(v) => assert_eq!(v, "value-of-s2"),
        other => panic!("expected String, got {}", other.type_name()),
    }
    let _ = handle; // the container stays usable after the forget
    put(&handle, "new-leaf", &[&s2]);
    assert_eq!(keys(&handle), vec!["new-leaf".to_string(), s2.clone()]);
}

#[test]
fn cascade_forget_from_leaf_deletes_only_the_leaf_branch() {
    let (handle, _h, s1, _s2, _mid, leaf) = build_office_graph("n351-c1b");
    let grant = issue_grant("memory:forget:*", 3600.0, "unlimited", None);
    let out = call_builtin("memory_forget_cascade", &[handle.clone(), s(&leaf), grant])
        .expect("leaf forget succeeds");
    assert_eq!(str_list(&struct_field(&out, "deleted")), vec![leaf.clone()]);
    let rest = keys(&handle);
    assert_eq!(rest.len(), 4, "s1/s2/mid/side all survive a leaf forget");
    assert!(rest.contains(&s1));
}

// ── C2: the preview matches the forget and consumes nothing ────────────

#[test]
fn preview_matches_the_subsequent_forget_without_side_effects() {
    let (handle, _h, s1, s2, _mid, _leaf) = build_office_graph("n351-c2");
    let before = keys(&handle);
    let preview = call_builtin("memory_cascade_preview", &[handle.clone(), s(&s1)])
        .expect("preview is read-only");
    let mut closure = str_list(&struct_field(&preview, "closure"));
    closure.sort();
    assert_eq!(closure, vec!["leaf", "mid", "s1", "side"]);
    assert!(str_list(&struct_field(&preview, "blocked_by")).is_empty());
    assert_eq!(keys(&handle), before, "preview changed nothing");
    // An unblocked preview means the forget deletes exactly the closure.
    let grant = issue_grant("memory:forget:*", 3600.0, "once", None);
    let out =
        call_builtin("memory_forget_cascade", &[handle.clone(), s(&s1), grant]).expect("forget ok");
    let mut deleted = str_list(&struct_field(&out, "deleted"));
    deleted.sort();
    assert_eq!(deleted, closure);
    assert_eq!(keys(&handle), vec![s2]);
}

// ── C3: the retained veto refuses the whole forget, consumes nothing ───

#[test]
fn retained_node_vetoes_the_cascade_and_keeps_the_grant_usable() {
    let (handle, _h, s1, _s2, _mid, _leaf) = build_office_graph("n351-c3");
    // retain(leaf) pins the leaf's (trivial) closure; the forget of s1
    // reaches leaf — the veto fires.
    call_builtin("memory_retain", &[handle.clone(), s("leaf")]).expect("retain ok");
    let retained =
        match call_builtin("memory_retained", std::slice::from_ref(&handle)).expect("retained") {
            Value::List(items) => items.len(),
            other => panic!("expected List, got {}", other.type_name()),
        };
    assert_eq!(retained, 1);
    let preview =
        call_builtin("memory_cascade_preview", &[handle.clone(), s(&s1)]).expect("preview");
    assert_eq!(
        str_list(&struct_field(&preview, "blocked_by")),
        vec!["leaf"]
    );

    // A Once grant consumed by a REFUSED forget would be theft — the
    // veto refuses BEFORE consumption; the same grant must stay usable
    // for the successful forget after the release.
    let grant = issue_grant("memory:forget:*", 3600.0, "once", None);
    let err = call_builtin(
        "memory_forget_cascade",
        &[handle.clone(), s(&s1), grant.clone()],
    )
    .expect_err("the veto refuses");
    assert!(
        err.contains("MEMORY_RETAIN_PROTECTED"),
        "typed refusal, got: {err}"
    );
    assert!(err.contains("leaf"), "the refusal NAMES the pin");
    assert_eq!(
        keys(&handle).len(),
        5,
        "nothing was deleted by the refused forget"
    );

    // Release, then the SAME (unconsumed) grant performs the forget.
    call_builtin("memory_release", &[handle.clone(), s("leaf")]).expect("release ok");
    let out = call_builtin("memory_forget_cascade", &[handle.clone(), s(&s1), grant])
        .expect("the same grant still works — it was never consumed");
    assert_eq!(str_list(&struct_field(&out, "deleted")).len(), 4);
    let _ = handle;
}

#[test]
fn retain_cascades_over_the_descendant_closure_and_survives_overwrite() {
    let (handle, _h, _s1, _s2, _mid, _leaf) = build_office_graph("n351-c4");
    // retain(s1) pins {s1, mid, leaf, side} — the whole descendant closure.
    call_builtin("memory_retain", &[handle.clone(), s("s1")]).expect("retain ok");
    let mut pinned = str_list(
        &call_builtin("memory_retained", std::slice::from_ref(&handle)).expect("retained"),
    );
    pinned.sort();
    assert_eq!(
        pinned,
        vec!["leaf", "mid", "s1", "side"],
        "the pin CASCADES"
    );

    // The pin belongs to the node identity: an overwrite keeps it.
    call_builtin(
        "memory_put",
        &[handle.clone(), s("mid"), s("rewritten"), list(&["s1"])],
    )
    .expect("overwrite ok");
    let mut pinned2 = str_list(
        &call_builtin("memory_retained", std::slice::from_ref(&handle)).expect("retained"),
    );
    pinned2.sort();
    assert_eq!(pinned2, vec!["leaf", "mid", "s1", "side"]);

    // release(s1) unpins the same closure (idempotent on repeat).
    call_builtin("memory_release", &[handle.clone(), s("s1")]).expect("release ok");
    call_builtin("memory_release", &[handle.clone(), s("s1")]).expect("release idempotent");
    assert!(str_list(
        &call_builtin("memory_retained", std::slice::from_ref(&handle)).expect("retained")
    )
    .is_empty());
}

// ── C5: the grant matrix (ADR-0155 §3.5) ───────────────────────────────

#[test]
fn forget_without_a_grant_refuses_with_grant_missing() {
    let (handle, _h, s1, _s2, _mid, _leaf) = build_office_graph("n351-c5a");
    let err = call_builtin(
        "memory_forget_cascade",
        &[handle.clone(), s(&s1), s("not-a-grant")],
    )
    .expect_err("a String is not a Grant");
    assert!(err.contains("GRANT_MISSING"), "typed refusal, got: {err}");
    assert_eq!(keys(&handle).len(), 5, "nothing deleted");
}

#[test]
fn forget_with_a_foreign_scope_refuses_with_grant_scope_mismatch() {
    let (handle, h, s1, _s2, _mid, _leaf) = build_office_graph("n351-c5b");
    let grant = issue_grant("db:delete:users", 3600.0, "unlimited", None);
    let err = call_builtin("memory_forget_cascade", &[handle.clone(), s(&s1), grant])
        .expect_err("db-scope does not cover memory:forget");
    assert!(err.contains("GRANT_SCOPE_MISMATCH"), "got: {err}");
    assert_eq!(keys(&handle).len(), 5);

    // The prefix scope covers only ITS container: a second container's
    // forget refuses under the first container's pinned scope.
    let subject2 = unique("n351-c5b-2");
    let handle2 = open_public(&subject2);
    let _h2 = handle_id(&handle2);
    put(&handle2, "x", &[]);
    let grant2 = issue_grant(&format!("memory:forget:{}", h), 3600.0, "unlimited", None);
    let err2 = call_builtin("memory_forget_cascade", &[handle2.clone(), s("x"), grant2])
        .expect_err("scope pins ONE container");
    assert!(err2.contains("GRANT_SCOPE_MISMATCH"), "got: {err2}");
}

#[test]
fn once_grant_is_linear_grant_reused_on_the_second_forget() {
    let subject = unique("n351-c5c");
    let handle = open_public(&subject);
    let _h = handle_id(&handle);
    put(&handle, "a", &[]);
    put(&handle, "b", &[]);
    let grant = issue_grant("memory:forget:*", 3600.0, "once", None);
    call_builtin(
        "memory_forget_cascade",
        &[handle.clone(), s("a"), grant.clone()],
    )
    .expect("the first Once use succeeds");
    let err = call_builtin("memory_forget_cascade", &[handle.clone(), s("b"), grant])
        .expect_err("a Once grant is linear");
    assert!(err.contains("GRANT_REUSED"), "got: {err}");
    assert_eq!(
        keys(&handle),
        vec!["b".to_string()],
        "the second forget never happened"
    );
}

#[test]
fn n_grant_exhausts_and_revoked_and_expired_refuse() {
    let subject = unique("n351-c5d");
    let handle = open_public(&subject);
    let h = handle_id(&handle);
    for k in ["a", "b", "c"] {
        put(&handle, k, &[]);
    }
    // N(2): two uses succeed, the third refuses with GRANT_EXHAUSTED.
    let grant = issue_grant("memory:forget:*", 3600.0, "n", Some(2.0));
    call_builtin(
        "memory_forget_cascade",
        &[handle.clone(), s("a"), grant.clone()],
    )
    .expect("use 1");
    call_builtin(
        "memory_forget_cascade",
        &[handle.clone(), s("b"), grant.clone()],
    )
    .expect("use 2");
    let err = call_builtin("memory_forget_cascade", &[handle.clone(), s("c"), grant])
        .expect_err("the quota is spent");
    assert!(err.contains("GRANT_EXHAUSTED"), "got: {err}");
    assert_eq!(keys(&handle), vec!["c".to_string()]);

    // Revoked → GRANT_REVOKED; expired → GRANT_EXPIRED (library level —
    // the ttl=0 grant is born expired, the refusal matrix precedent).
    let g_revoked = issue_grant("memory:forget:*", 3600.0, "unlimited", None);
    call_builtin("grant_revoke", std::slice::from_ref(&g_revoked)).expect("revoke ok");
    let err_rev = call_builtin(
        "memory_forget_cascade",
        &[handle.clone(), s("c"), g_revoked],
    )
    .expect_err("revoked");
    assert!(err_rev.contains("GRANT_REVOKED"), "got: {err_rev}");

    let born_expired =
        grants::issue("memory:forget:*", 0, &GrantClass::Unlimited, "program").expect("issue ok");
    let err_exp = memory_typed::forget_cascade(&h, "c", &born_expired).expect_err("born-expired");
    assert!(err_exp.contains("GRANT_EXPIRED"), "got: {err_exp}");
    assert_eq!(
        keys(&handle),
        vec!["c".to_string()],
        "no deletion ever happened"
    );
}

// ── C6: the post-success journal ────────────────────────────────────────

#[test]
fn the_journal_records_the_forget_without_the_values() {
    let subject = unique("n351-c6");
    let handle = open_public(&subject);
    let h = handle_id(&handle);
    put(&handle, "root", &[]);
    put(&handle, "child", &["root"]);
    let grant = issue_grant("memory:forget:*", 3600.0, "unlimited", None);
    let out = call_builtin("memory_forget_cascade", &[handle.clone(), s("root"), grant])
        .expect("forget ok");
    let batch = str_field(&out, "batch_id");
    assert!(
        batch.starts_with("MLOG-TFORGET-"),
        "the №280 batch posture: {batch}"
    );

    let records = all_records().expect("ledger readable");
    // LedgerRecord stores NO detail prose (the Task-7 lesson) — the
    // operation is identified by the action family + actor + scope, and
    // the payload survives only as args_hash (the values never journal).
    let hit = records
        .iter()
        .find(|r| r.action == "irreversible.memory_forget" && r.scope == "memory:forget:*")
        .expect("the post-success record exists");
    assert_eq!(hit.actor, "program", "the record names the grant issuer");
    assert!(
        !hit.args_hash.is_empty(),
        "the digest side went into args_hash"
    );
    // The memory.* family trail also carries the operation (actor = container).
    assert!(records
        .iter()
        .any(|r| r.action == "memory.forget_cascade" && r.actor == h));
}

// ── The library-level pure plan (the fuzzer drives the same fn) ────────

#[test]
fn plan_cascade_is_a_pure_function_over_nodes_edges_pins() {
    use metalogos::memory_typed::plan_cascade;
    use std::collections::{HashMap, HashSet};
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    children.insert("root".into(), vec!["a".into(), "b".into()]);
    children.insert("a".into(), vec!["leaf".into()]);
    let retained: HashSet<String> = HashSet::new();
    let plan = plan_cascade("root", &children, &retained);
    assert_eq!(plan.closure, vec!["a", "b", "leaf", "root"]);
    assert!(plan.blocked_by.is_empty());
    // O() counters: exactly one pop per closure node, one scan per edge.
    assert_eq!(plan.visited_nodes, 4);
    assert_eq!(plan.visited_edges, 3);
    // Determinism: the same input, the same plan.
    let again = plan_cascade("root", &children, &retained);
    assert_eq!(plan, again);
}

#[test]
fn graph_class_entries_exist_for_all_five_surfaces() {
    for name in [
        "memory_cascade_preview",
        "memory_retain",
        "memory_release",
        "memory_retained",
        "memory_forget_cascade",
    ] {
        assert!(
            BUILTIN_REGISTRY.iter().any(|sp| sp.name == name),
            "{name} must be registered"
        );
    }
    // The delete-class posture of the forget (№316): Sink × Irreversible.
    let cls =
        metalogos::builtins_classification::classify("memory_forget_cascade").expect("classified");
    assert_eq!(cls.role, metalogos::builtins_classification::Role::Sink);
    assert_eq!(
        cls.reversibility,
        metalogos::builtins_classification::Reversibility::Irreversible
    );
}

// GrantClass sanity used by the matrix above (the ADR-0155 algebra).
#[test]
fn grant_class_ledger_forms_are_stable() {
    assert_eq!(GrantClass::Once.as_ledger_str(), "once");
    assert_eq!(GrantClass::N(3).as_ledger_str(), "n:3");
    assert_eq!(GrantClass::Unlimited.as_ledger_str(), "unlimited");
}
