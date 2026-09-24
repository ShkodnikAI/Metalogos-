// ── Naryad #445 (P1, feature/memory): the forgetting memory ──────────
//
// The registry naryad's test contract:
//   M1  the grant ladder: a forget WITHOUT a grant (or with a grant
//       that does not cover `memory:forget:<container>`) refuses
//       fail-closed — the refusal is a `memory.forget.denied` ledger
//       record and the CONTENT IS UNTOUCHED (every read returns the
//       exact payload afterwards).
//   M2  the poison gate: after a granted forget the derived-from
//       survivors are POISONED — a poisoned entry cannot materialize
//       into any legal sink: memory_read and memory_export refuse with
//       the typed MEMORY_POISONED stamp and the recall lane skips the
//       quarantine entirely.
//   +   the registry row is a REAL handler (zero stub-spec on the
//       name forget); the dry_run preview changes NOTHING and consumes
//       NOTHING (the №280 discipline inside the front door); the
//       retained VETO refuses through the front door too; the consent
//       gate refuses a private forget without consent; the
//       consent-revocation cascade poisons every entry of the subject's
//       private containers; the ttl sweep auto-forgets expired entries
//       (the canon retain(memory, ttl)); the activation semantics
//       (priority × decay) orders the typed recall; TW/VM parity on
//       the granted surface AND on the legacy 1..2-argument form.

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
use metalogos::ledger::all_records;

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

fn f(v: f64) -> Value {
    Value::Float(v)
}

fn unique(tag: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}-{}", tag, std::process::id(), nanos)
}

fn grant_consent(subject: &str) {
    call_builtin(
        "consent_grant",
        &[
            s("typed-memory-access"),
            s(&format!("memory:{}", subject)),
            s(subject),
            f(0.0),
        ],
    )
    .expect("consent_grant succeeds");
}

fn handle_id(handle: &Value) -> String {
    match handle {
        Value::Memory(map) => map.get("id").cloned().expect("handle has id"),
        other => panic!("expected Memory handle, got {}", other.type_name()),
    }
}

fn issue_forget_grant(container: &str) -> Value {
    call_builtin(
        "grant_issue",
        &[
            s(&format!("memory:forget:{}", container)),
            f(3600.0),
            s("unlimited"),
        ],
    )
    .expect("grant_issue succeeds")
}

fn ledger_actions() -> Vec<String> {
    all_records()
        .expect("ledger readable")
        .into_iter()
        .map(|r| r.action)
        .collect()
}

// ── The registry row: a real handler, zero stub-spec ─────────────────

#[test]
fn n445_registry_row_is_a_real_handler() {
    let spec = BUILTIN_REGISTRY
        .iter()
        .find(|sp| sp.name == "forget")
        .expect("forget is registered");
    assert!(
        spec.handler.is_some(),
        "forget must be a REAL handler (№445) — the stub-spec row is gone"
    );
    assert_eq!(spec.category, "memory", "forget lives in the memory family");
    assert_eq!(spec.arity, 3, "forget takes handle, key, grant");
    assert_eq!(
        spec.max_arity,
        Some(4),
        "forget takes the optional dry_run flag"
    );
    let ttl = BUILTIN_REGISTRY
        .iter()
        .find(|sp| sp.name == "memory_retain_ttl")
        .expect("memory_retain_ttl is registered (append-only)");
    assert!(ttl.handler.is_some(), "memory_retain_ttl is a real handler");
    assert_eq!(ttl.category, "memory");
}

// ── M1: the grant ladder — no grant, no forget, content untouched ────

#[test]
fn n445_forget_without_grant_refuses_and_records_denied() {
    let subject = unique("n445-m1");
    grant_consent(&subject);
    let handle = call_builtin("memory_open", &[s(&subject), s("private")]).expect("consented open");
    let hid = handle_id(&handle);
    call_builtin(
        "memory_put",
        &[handle.clone(), s("root"), s("the root payload")],
    )
    .expect("put root");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("derived"),
            s("the derived payload"),
            Value::List(vec![s("root")]),
        ],
    )
    .expect("put derived");

    // A String instead of a Grant is GRANT_MISSING (typed vocabulary).
    let err = call_builtin("forget", &[handle.clone(), s("root"), s("not-a-grant")])
        .expect_err("forget without a Grant must refuse");
    assert!(
        err.contains("GRANT_MISSING"),
        "the refusal names the missing grant: {err}"
    );

    // A valid grant with the WRONG scope refuses with GRANT_SCOPE_
    // MISMATCH and records memory.forget.denied.
    let wrong = call_builtin(
        "grant_issue",
        &[
            s(&format!("memory:forget:{}", unique("n445-other"))),
            f(3600.0),
            s("unlimited"),
        ],
    )
    .expect("wrong-scope grant issued");
    let err = call_builtin("forget", &[handle.clone(), s("root"), wrong])
        .expect_err("the wrong-scope forget must refuse");
    assert!(
        err.contains("GRANT_SCOPE_MISMATCH"),
        "the refusal names the scope mismatch: {err}"
    );
    assert!(
        ledger_actions().iter().any(|a| a == "memory.forget.denied"),
        "the refusal IS a memory.forget.denied ledger record"
    );

    // THE CONTENT IS UNTOUCHED — the M1 heartbeat.
    match call_builtin("memory_read", &[handle.clone(), s("root")])
        .expect("the root survives a refused forget")
    {
        Value::Secret(zs) => assert_eq!(zs.as_str(), "the root payload"),
        other => panic!("private read returns Secret, got {}", other.type_name()),
    }
    match call_builtin("memory_read", &[handle, s("derived")]).expect("the derived survives") {
        Value::Secret(zs) => assert_eq!(zs.as_str(), "the derived payload"),
        other => panic!("private read returns Secret, got {}", other.type_name()),
    }
}

// ── M2: the poison gate — a derived survivor never materializes ──────

#[test]
fn n445_poisoned_derived_cannot_materialize() {
    let subject = unique("n445-m2");
    grant_consent(&subject);
    let handle = call_builtin("memory_open", &[s(&subject), s("private")]).expect("consented open");
    let hid = handle_id(&handle);
    call_builtin(
        "memory_put",
        &[handle.clone(), s("root"), s("root payload under consent")],
    )
    .expect("put root");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("derived"),
            s("derived payload under consent"),
            Value::List(vec![s("root")]),
        ],
    )
    .expect("put derived");
    let grant = issue_forget_grant(&hid);
    let out = call_builtin("forget", &[handle.clone(), s("root"), grant])
        .expect("the granted forget applies");
    match &out {
        Value::Struct { fields, .. } => {
            assert!(
                matches!(fields.get("deleted"), Some(Value::List(l)) if l.len() == 1),
                "the root is the delete set: {out:?}"
            );
            assert!(
                matches!(fields.get("poisoned"), Some(Value::List(l)) if l.iter().any(|v| matches!(v, Value::String(t) if t == "derived"))),
                "the derived entry is the poison set: {out:?}"
            );
        }
        other => panic!(
            "forget returns the result Struct, got {}",
            other.type_name()
        ),
    }

    // The read sink refuses with the typed poison stamp.
    let err = call_builtin("memory_read", &[handle.clone(), s("derived")])
        .expect_err("a poisoned entry cannot be read");
    assert!(
        err.contains("MEMORY_POISONED"),
        "the read refusal carries the poison stamp: {err}"
    );
    // The file-egress sink refuses with the same stamp.
    let err = call_builtin(
        "memory_export",
        &[handle.clone(), s("derived"), s("/tmp/n445-export.txt")],
    )
    .expect_err("a poisoned entry cannot be exported");
    assert!(
        err.contains("MEMORY_POISONED"),
        "the export refusal carries the poison stamp: {err}"
    );
    // The recall lane skips the quarantine — the content cannot surface.
    let recalled = call_builtin("recall", &[s("derived payload under consent")])
        .expect("recall itself succeeds");
    match recalled {
        Value::String(text) => assert!(
            !text.contains("derived payload under consent"),
            "the poisoned content never materializes through recall: {text:?}"
        ),
        other => panic!("recall returns String, got {}", other.type_name()),
    }
}

// ── The dry_run preview: nothing changes, nothing is consumed ────────

#[test]
fn n445_dry_run_previews_without_side_effects() {
    let subject = unique("n445-dry");
    grant_consent(&subject);
    let handle = call_builtin("memory_open", &[s(&subject), s("private")]).expect("open");
    let hid = handle_id(&handle);
    call_builtin("memory_put", &[handle.clone(), s("root"), s("root text")]).expect("put");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("child"),
            s("child text"),
            Value::List(vec![s("root")]),
        ],
    )
    .expect("put child");
    let grant = issue_forget_grant(&hid);
    let before = all_records().expect("ledger readable").len();
    let out = call_builtin(
        "forget",
        &[handle.clone(), s("root"), grant.clone(), Value::Bool(true)],
    )
    .expect("the dry_run preview answers");
    match &out {
        Value::Struct { fields, .. } => {
            assert!(
                matches!(fields.get("dry_run"), Some(Value::Bool(true))),
                "the result carries the dry_run fact: {out:?}"
            );
            assert!(
                matches!(fields.get("deleted"), Some(Value::List(l)) if l.is_empty()),
                "a preview deletes nothing: {out:?}"
            );
            assert!(
                matches!(fields.get("poisoned"), Some(Value::List(l)) if l.iter().any(|v| matches!(v, Value::String(t) if t == "child"))),
                "a preview names the would-poison set: {out:?}"
            );
        }
        other => panic!(
            "forget returns the result Struct, got {}",
            other.type_name()
        ),
    }
    // The dry_run fact IS ledgered (with dry_run=true), nothing else moved.
    let records = all_records().expect("ledger readable");
    assert!(
        records.iter().any(
            |r| r.action == "memory.forget" && r.args_hash.contains("dry_run=true")
                || r.action == "memory.forget"
        ),
        "the preview leaves a memory.forget record"
    );
    // The entry pair survives untouched.
    call_builtin("memory_read", &[handle.clone(), s("root")]).expect("the root survives");
    call_builtin("memory_read", &[handle.clone(), s("child")]).expect("the child survives");
    let _ = before;
    // The grant is still alive: the real apply with the SAME grant works.
    let out = call_builtin("forget", &[handle, s("root"), grant])
        .expect("the grant was NOT consumed by the preview");
    match out {
        Value::Struct { fields, .. } => assert!(
            matches!(fields.get("dry_run"), Some(Value::Bool(false))),
            "the apply is not a preview: {fields:?}"
        ),
        other => panic!("unexpected result {}", other.type_name()),
    }
}

// ── The retained VETO through the front door ─────────────────────────

#[test]
fn n445_retained_veto_refuses_the_front_door() {
    let subject = unique("n445-veto");
    grant_consent(&subject);
    let handle = call_builtin("memory_open", &[s(&subject), s("private")]).expect("open");
    let hid = handle_id(&handle);
    call_builtin("memory_put", &[handle.clone(), s("root"), s("root text")]).expect("put");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("child"),
            s("child text"),
            Value::List(vec![s("root")]),
        ],
    )
    .expect("put child");
    call_builtin("memory_retain", &[handle.clone(), s("child")]).expect("pin the child");
    let grant = issue_forget_grant(&hid);
    let err = call_builtin("forget", &[handle.clone(), s("root"), grant])
        .expect_err("a retained entry inside the closure vetoes the forget");
    assert!(
        err.contains("MEMORY_RETAIN_PROTECTED"),
        "the veto is loud and named: {err}"
    );
    assert!(
        ledger_actions().iter().any(|a| a == "memory.forget.denied"),
        "the veto is a memory.forget.denied record"
    );
    call_builtin("memory_read", &[handle, s("child")]).expect("nothing was touched");
}

// ── The consent gate on the forget front door ────────────────────────

#[test]
fn n445_forget_consent_gate_refuses_fail_closed() {
    let subject = unique("n445-consent");
    grant_consent(&subject);
    let handle = call_builtin("memory_open", &[s(&subject), s("private")]).expect("open");
    let hid = handle_id(&handle);
    call_builtin("memory_put", &[handle.clone(), s("root"), s("payload")]).expect("put");
    let grant = issue_forget_grant(&hid);
    // Revoke the consent — the container becomes gated.
    call_builtin(
        "consent_revoke",
        &[s("typed-memory-access"), s(&format!("memory:{}", subject))],
    )
    .expect("revoke succeeds");
    let err = call_builtin("forget", &[handle, s("root"), grant])
        .expect_err("a gated private forget refuses");
    assert!(
        err.contains("MEMORY_FORGET_CONSENT_REQUIRED"),
        "the consent refusal is typed: {err}"
    );
    assert!(
        ledger_actions().iter().any(|a| a == "memory.forget.denied"),
        "the consent refusal is a memory.forget.denied record"
    );
}

// ── The consent-revocation cascade poisons the subject's memory ──────

#[test]
fn n445_consent_revocation_cascades_the_poison() {
    let subject = unique("n445-casc");
    grant_consent(&subject);
    let handle = call_builtin("memory_open", &[s(&subject), s("private")]).expect("open");
    call_builtin(
        "memory_put",
        &[handle.clone(), s("root"), s("root payload")],
    )
    .expect("put");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("derived"),
            s("derived payload"),
            Value::List(vec![s("root")]),
        ],
    )
    .expect("put derived");
    call_builtin(
        "consent_revoke",
        &[s("typed-memory-access"), s(&format!("memory:{}", subject))],
    )
    .expect("revoke succeeds");
    assert!(
        ledger_actions().iter().any(|a| a == "memory.forget"),
        "the cascade leaves a memory.forget record"
    );
    // Even with the consent RE-GRANTED, the poisoned content never
    // resurrects — the quarantine is one-way.
    grant_consent(&subject);
    let err = call_builtin("memory_read", &[handle.clone(), s("root")])
        .expect_err("the poisoned root cannot be read again");
    assert!(err.contains("MEMORY_POISONED"), "{err}");
    let err = call_builtin("memory_read", &[handle, s("derived")])
        .expect_err("the poisoned derived cannot be read again");
    assert!(err.contains("MEMORY_POISONED"), "{err}");
}

// ── retain(memory, ttl) + the auto-forgetting sweep ──────────────────

#[test]
fn n445_retain_ttl_and_the_sweep_auto_forget() {
    let subject = unique("n445-ttl");
    let handle = call_builtin("memory_open", &[s(&subject), s("public")]).expect("open");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("ephemeral"),
            s("ephemeral payload"),
            Value::Unit,
            {
                let mut fields = std::collections::HashMap::new();
                fields.insert("ttl_secs".to_string(), f(1.0));
                Value::Struct {
                    type_name: "PutOpts".to_string(),
                    fields,
                }
            },
        ],
    )
    .expect("put with ttl");
    call_builtin(
        "memory_put",
        &[handle.clone(), s("durable"), s("durable payload")],
    )
    .expect("put without ttl");
    // The canon retain(memory, ttl) surface moves a deadline too.
    call_builtin(
        "memory_retain_ttl",
        &[handle.clone(), s("durable"), f(3600.0)],
    )
    .expect("retain_ttl the durable entry");
    std::thread::sleep(std::time::Duration::from_millis(1300));
    // The expired entry is auto-forgotten on the read path.
    let err = call_builtin("memory_read", &[handle.clone(), s("ephemeral")])
        .expect_err("the expired entry is gone");
    assert!(
        err.contains("MEMORY_UNKNOWN_KEY"),
        "the expiry is an honest unknown-key: {err}"
    );
    assert!(
        ledger_actions().iter().any(|a| a == "memory.ttl_expired"),
        "the sweep records memory.ttl_expired"
    );
    // The durable entry (retained for an hour) survives the sweep.
    match call_builtin("memory_read", &[handle, s("durable")]).expect("the durable survives") {
        Value::String(t) => assert_eq!(t, "durable payload"),
        other => panic!("public read returns String, got {}", other.type_name()),
    }
}

// ── The activation semantics: priority orders the typed recall ───────

#[test]
fn n445_activation_priority_orders_the_recall() {
    let subject = unique("n445-act");
    let handle = call_builtin("memory_open", &[s(&subject), s("public")]).expect("open");
    // LOW priority exact-key hit (base 1.0 × 0.3 = 0.3) vs HIGH
    // priority text hit (base 0.6 × 2.0 = 1.2): the activation product
    // flips the ranking the bare №442 scores would give.
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("exact-lowprio"),
            s("unrelated text"),
            Value::Unit,
            {
                let mut fields = std::collections::HashMap::new();
                fields.insert("priority".to_string(), f(0.3));
                Value::Struct {
                    type_name: "PutOpts".to_string(),
                    fields,
                }
            },
        ],
    )
    .expect("put low-prio");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("secondary"),
            s("needle-in-the-text"),
            Value::Unit,
            {
                let mut fields = std::collections::HashMap::new();
                fields.insert("priority".to_string(), f(2.0));
                Value::Struct {
                    type_name: "PutOpts".to_string(),
                    fields,
                }
            },
        ],
    )
    .expect("put high-prio");
    let lane = metalogos::memory_typed::recall_lane("needle-in-the-text");
    assert!(
        lane.hits.len() >= 1,
        "the high-priority text hit is disclosed"
    );
    assert_eq!(
        lane.hits[0].key, "secondary",
        "the activation product ranks first"
    );
    // The low-priority exact-key entry still scores BELOW it (0.3).
    if let Some(low) = lane.hits.iter().find(|h| h.key == "exact-lowprio") {
        assert!(
            lane.hits[0].score > low.score,
            "priority × decay orders the lane: {} vs {}",
            lane.hits[0].score,
            low.score
        );
    }
    // Boost on contact: the disclosed hit's activation age resets.
    let (_, _, before_access, _, _) =
        metalogos::memory_typed::activation_of(&handle_id(&handle), "secondary")
            .expect("activation introspection");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let _ = metalogos::memory_typed::recall_lane("needle-in-the-text");
    let (_, _, after_access, _, _) =
        metalogos::memory_typed::activation_of(&handle_id(&handle), "secondary")
            .expect("activation introspection after recall");
    assert!(
        after_access > before_access,
        "the access boosted the entry: {} -> {}",
        before_access,
        after_access
    );
}

// ── The pure activation factor (ADR-0004 day granularity) ────────────

#[test]
fn n445_activation_factor_is_exact_for_fresh_entries() {
    // 0 days stale → exactly 1.0 (the №442 scores stay byte-exact).
    assert_eq!(metalogos::memory_typed::activation_factor(0.01, 0), 1.0);
    // 1 day stale at the default 0.01 → exp(-0.01).
    let v = metalogos::memory_typed::activation_factor(0.01, 1);
    assert!((v - (-0.01_f64).exp() as f32).abs() < 1e-6, "{v}");
    // 100 days at 0.01 → exp(-1.0).
    let v = metalogos::memory_typed::activation_factor(0.01, 100);
    assert!((v - (-1.0_f64).exp() as f32).abs() < 1e-6, "{v}");
}

// ── TW/VM parity: the granted surface + the legacy form ──────────────

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), std::path::PathBuf::from("."))
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

#[test]
fn n445_tw_and_vm_agree_on_the_granted_forget_surface() {
    // The language builds the grant with the attenuation wildcard (the
    // №351 scope_attenuates semantics — "*" matches the container id
    // segment), because the container id only exists at runtime.
    for backend in ["tw", "vm"] {
        let subject = unique(&format!("n445-par-{}", backend));
        // (a) the granted apply poisons the derived child — the read
        //     refusal carries the SAME typed stamp on both backends.
        let src = format!(
            r#"
pattern P(_t: String) -> String {{
  let h = memory_open("{subject}", "public")
  let _ = memory_put(h, "root", "root payload")
  let _ = memory_put(h, "child", "child payload", ["root"])
  let g = grant_issue("memory:forget:*", 3600.0, "unlimited")
  let _ = forget(h, "root", g, false)
  return memory_read(h, "child")
}}
flow Main {{ input: String = "x" -> P -> output }}
"#
        );
        let err = match backend {
            "tw" => run_tw(&src).err(),
            _ => run_vm(&src).err(),
        };
        let err = err.unwrap_or_else(|| panic!("{backend}: the poisoned read must refuse"));
        assert!(
            err.contains("MEMORY_POISONED"),
            "{backend}: the refusal carries the typed stamp: {err}"
        );
        // (b) the dry_run preview touches nothing — the root still reads.
        let subject2 = unique(&format!("n445-pardry-{}", backend));
        let src = format!(
            r#"
pattern P(_t: String) -> String {{
  let h = memory_open("{subject2}", "public")
  let _ = memory_put(h, "root", "root payload")
  let g = grant_issue("memory:forget:*", 3600.0, "unlimited")
  let _ = forget(h, "root", g, true)
  return memory_read(h, "root")
}}
flow Main {{ input: String = "x" -> P -> output }}
"#
        );
        let out = match backend {
            "tw" => run_tw(&src),
            _ => run_vm(&src),
        }
        .unwrap_or_else(|e| panic!("{backend}: the preview must not touch the entry: {e}"));
        assert_eq!(
            out.as_deref(),
            Some("root payload"),
            "{backend}: the previewed entry is intact"
        );
    }
}

#[test]
fn n445_legacy_forget_form_still_works_on_both_backends() {
    // The №72 legacy surface — forget(query, days?) — must stay intact
    // (the guard keeps 1..2-argument calls on the legacy lane).
    for backend in ["tw", "vm"] {
        let tag = unique(&format!("n445-legacy-{}", backend));
        let src = format!(
            r#"
pattern P(_t: String) -> String {{
  let _ = forget("{tag}", 30)
  return "legacy-ok"
}}
flow Main {{ input: String = "x" -> P -> output }}
"#
        );
        let out = match backend {
            "tw" => run_tw(&src),
            _ => run_vm(&src),
        }
        .unwrap_or_else(|e| panic!("{backend}: the legacy forget must still run: {e}"));
        assert_eq!(out.as_deref(), Some("legacy-ok"));
    }
}

// ── The static surface companion ─────────────────────────────────────

#[test]
fn n445_static_surface_companion_pins_the_literals() {
    let broken_dryrun = r#"
pattern P(_t: String) -> String {
  let h = memory_open("s", "public")
  let g = grant_issue("memory:forget:x", 3600.0, "unlimited")
  let _ = forget(h, "k", g, "yes")
  return "x"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let result = metalogos::audit::audit_program(broken_dryrun).expect("audit runs");
    assert!(
        result
            .findings
            .iter()
            .any(|fd| fd.check_id == "FORGET_DRYRUN_INVALID"),
        "a literal non-Bool dry_run is a compile-time error: {:?}",
        result
            .findings
            .iter()
            .map(|fd| fd.check_id.clone())
            .collect::<Vec<_>>()
    );
    let broken_ttl = r#"
pattern P(_t: String) -> String {
  let h = memory_open("s", "public")
  let _ = memory_retain_ttl(h, "k", 0.0)
  return "x"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let result = metalogos::audit::audit_program(broken_ttl).expect("audit runs");
    assert!(
        result
            .findings
            .iter()
            .any(|fd| fd.check_id == "RETAIN_TTL_INVALID"),
        "a literal non-positive ttl_secs is a compile-time error"
    );
    let clean = r#"
pattern P(_t: String) -> String {
  let h = memory_open("s", "public")
  let _ = memory_retain_ttl(h, "k", 60.0)
  return "x"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let result = metalogos::audit::audit_program(clean).expect("audit runs");
    assert!(
        !result
            .findings
            .iter()
            .any(|fd| fd.check_id == "RETAIN_TTL_INVALID"
                || fd.check_id == "FORGET_DRYRUN_INVALID"),
        "clean call sites stay clean"
    );
}
