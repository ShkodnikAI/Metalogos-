// ── Naryad #442 (P1, feature/memory): recall — the front door of memory ──
//
// The registry naryad's test contract:
//   M1  the consent gate: a recall query that names gated private
//       memory (a private container's key with no active grant)
//       refuses fail-closed with the typed MEMORY_RECALL_CONSENT_
//       REQUIRED stamp — and the refusal NEVER carries the content;
//       the refusal itself is a memory.recall.denied ledger record.
//   M2  the provenance: a typed-lane recall hit CARRIES its
//       provenance — the [MEM] suffix names the exact container id,
//       subject, label, time and the taint projection; losing or
//       substituting any of these is caught here.
//   +   the registry row is a REAL handler (zero stub-spec on the
//       name); the store lane's external contract is unchanged
//       (memorize → recall returns the exact value); the ledger
//       memory.recall family records {query hash, containers, hits,
//       consent fact}; TW/VM parity on the full surface; the static
//       recall surface companion (RECALL_QUERY_INVALID /
//       RECALL_CONFIDENCE_INVALID).

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
            Value::Float(0.0),
        ],
    )
    .expect("consent_grant succeeds");
}

fn revoke_consent(subject: &str) {
    call_builtin(
        "consent_revoke",
        &[s("typed-memory-access"), s(&format!("memory:{}", subject))],
    )
    .expect("consent_revoke succeeds");
}

fn handle_id(handle: &Value) -> String {
    match handle {
        Value::Memory(map) => map.get("id").cloned().expect("handle has id"),
        other => panic!("expected Memory handle, got {}", other.type_name()),
    }
}

fn ledger_actions() -> Vec<String> {
    all_records()
        .expect("ledger readable")
        .into_iter()
        .map(|r| r.action)
        .collect()
}

fn ledger_snapshot(action: &str) -> Vec<(String, String)> {
    all_records()
        .expect("ledger readable")
        .into_iter()
        .filter(|r| r.action == action)
        .map(|r| (r.actor, r.args_hash))
        .collect()
}

// ── The registry row: a real handler, zero stub-spec ─────────────────

#[test]
fn n442_registry_row_is_a_real_handler() {
    let spec = BUILTIN_REGISTRY
        .iter()
        .find(|sp| sp.name == "recall")
        .expect("recall is registered");
    assert!(
        spec.handler.is_some(),
        "recall must be a REAL handler (№442) — the stub-spec row is gone"
    );
    assert_eq!(spec.category, "memory", "recall lives in the memory family");
    assert_eq!(spec.arity, 1, "recall takes at least the query");
    assert_eq!(
        spec.max_arity,
        Some(2),
        "recall takes query + min_confidence"
    );
}

// ── M2: the typed-lane hit carries its provenance ────────────────────

#[test]
fn n442_typed_lane_hit_carries_provenance() {
    let subject = unique("n442-prov");
    grant_consent(&subject);
    let handle =
        call_builtin("memory_open", &[s(&subject), s("private")]).expect("consented private open");
    let hid = handle_id(&handle);
    // The parent first — derived-from is validated fail-closed (№351).
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("intake-form"),
            s("intake recorded"),
            Value::Unit,
        ],
    )
    .expect("parent put");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("medical-notes"),
            s("patient allergic to penicillin"),
            Value::List(vec![s("intake-form")]),
        ],
    )
    .expect("put");

    let recalled = call_builtin("recall", &[s("medical-notes")]).expect("granted recall");
    let text = match &recalled {
        Value::String(t) => t.clone(),
        other => panic!("recall must return String, got {}", other.type_name()),
    };

    // The content is disclosed (the grant covers it)…
    assert!(
        text.contains("patient allergic to penicillin"),
        "granted private content must be recallable, got: {text}"
    );
    // …and the provenance is CARRIED: exact container, subject, label,
    // taint projection, derived-from parents. Losing or substituting
    // any of these fails this test (the M2 mutation target).
    assert!(
        text.contains(&format!("[MEM] container={}", hid)),
        "the [MEM] suffix must name the EXACT container id {hid}: {text}"
    );
    assert!(
        text.contains(&format!("subject={}", subject)),
        "the [MEM] suffix must name the subject: {text}"
    );
    assert!(
        text.contains("label=private"),
        "the [MEM] suffix must carry the container label: {text}"
    );
    assert!(
        text.contains("labels=private, trusted"),
        "the taint projection of private memory is the Secret kind: {text}"
    );
    assert!(
        text.contains("derived_from=intake-form"),
        "the [MEM] suffix must carry the derived-from parents: {text}"
    );
    assert!(
        text.contains("time="),
        "the [MEM] suffix must carry time: {text}"
    );
}

#[test]
fn n442_public_container_hit_needs_no_consent() {
    let subject = unique("n442-pub");
    let handle = call_builtin("memory_open", &[s(&subject), s("public")])
        .expect("public open needs no consent");
    call_builtin(
        "memory_put",
        &[
            handle,
            s("team-handbook"),
            s("standup is at 10:00"),
            Value::Unit,
        ],
    )
    .expect("put");

    let recalled =
        call_builtin("recall", &[s("team-handbook")]).expect("public recall needs no consent");
    let text = match &recalled {
        Value::String(t) => t.clone(),
        other => panic!("recall must return String, got {}", other.type_name()),
    };
    assert!(text.contains("standup is at 10:00"), "hit text: {text}");
    assert!(text.contains("label=public"), "public hit label: {text}");
    assert!(
        !text.contains("labels=private"),
        "a public hit must NOT project the Secret taint: {text}"
    );
}

// ── M1: the fail-closed consent gate ─────────────────────────────────

#[test]
fn n442_gated_recall_refuses_fail_closed() {
    let subject = unique("n442-gate");
    grant_consent(&subject);
    let handle =
        call_builtin("memory_open", &[s(&subject), s("private")]).expect("consented private open");
    let hid = handle_id(&handle);
    call_builtin(
        "memory_put",
        &[
            handle,
            s("therapy-log"),
            s("session notes are strictly confidential"),
            Value::Unit,
        ],
    )
    .expect("put");

    // The grant is revoked — the address is now gated.
    revoke_consent(&subject);

    let refused = call_builtin("recall", &[s("therapy-log")]);
    let err = match refused {
        Err(e) => e,
        Ok(v) => panic!(
            "recalling gated private memory must REFUSE, got Ok({})",
            v.type_name()
        ),
    };

    // The typed origin-stamp (№413 — branchable at the try sewing points).
    assert!(
        err.contains("[MEMORY_RECALL_CONSENT_REQUIRED]"),
        "the refusal must carry the typed stamp: {err}"
    );
    // Fail-closed: the refusal NEVER carries the gated content (M1).
    assert!(
        !err.contains("session notes"),
        "the refusal must not leak the gated content: {err}"
    );
    // The refusal IS a ledger record — attributable to the gated
    // container (the actor IS the container id) with a hashed payload.
    let denied = ledger_snapshot("memory.recall.denied");
    assert!(
        denied.iter().any(|(actor, hash)| actor == &hid && !hash.is_empty()),
        "the refusal must record memory.recall.denied with the gated container as the actor, got: {:?}",
        denied
    );
}

#[test]
fn n442_granted_recall_is_a_ledger_record() {
    let subject = unique("n442-ledger");
    grant_consent(&subject);
    let handle =
        call_builtin("memory_open", &[s(&subject), s("private")]).expect("consented private open");
    call_builtin(
        "memory_put",
        &[
            handle,
            s("allergy-card"),
            s("penicillin allergy, anaphylaxis risk"),
            Value::Unit,
        ],
    )
    .expect("put");

    let before = ledger_snapshot("memory.recall").len();
    call_builtin("recall", &[s("allergy-card")]).expect("granted recall");
    let after = ledger_snapshot("memory.recall").len();

    assert!(
        after > before,
        "every recall call must leave a memory.recall record (before {before}, after {after})"
    );
    let records = ledger_snapshot("memory.recall");
    assert!(
        records.iter().any(|(actor, hash)| actor == "recall" && !hash.is_empty()),
        "the memory.recall record is the recall surface's own (actor=recall) with a hashed payload {{query hash, containers, hits, consent fact}}, got: {:?}",
        records
    );
}

// ── The store lane's external contract is unchanged ──────────────────

#[test]
fn n442_store_lane_contract_unchanged() {
    let source = r#"
memorize "user likes spicy food" with priority=0.9

pattern FindFood(query: String) -> String {
  return recall(query)
}

flow Main { input: String = "spicy" -> FindFood -> output }
"#;
    let out = metalogos::run_program(source).expect("the store lane recall runs");
    assert_eq!(
        out,
        Some("user likes spicy food".to_string()),
        "the store lane's contract is regression-pinned: the exact value, no [MEM] suffix"
    );
}

// ── TW/VM parity: the full surface on both backends ──────────────────

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

const PARITY_PROG: &str = r#"
pattern P(_t: String) -> String {
  let _ = consent_grant("c", "memory:PARITYSUBJ", "PARITYSUBJ", 0.0)
  let h = memory_open("PARITYSUBJ", "private")
  let _ = memory_put(h, "parity-key", "parity payload under consent")
  return recall("parity-key")
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn n442_tw_and_vm_agree_on_granted_recall() {
    let tw_prog = PARITY_PROG.replace("PARITYSUBJ", &unique("n442-par-tw"));
    let vm_prog = PARITY_PROG.replace("PARITYSUBJ", &unique("n442-par-vm"));

    for (backend, src) in [("tw", tw_prog), ("vm", vm_prog)] {
        let out = match backend {
            "tw" => run_tw(&src),
            _ => run_vm(&src),
        }
        .unwrap_or_else(|e| panic!("{backend}: the granted recall must run: {e}"));
        let text = out.expect("{backend}: the flow returns the recall result");
        assert!(
            text.contains("parity payload under consent"),
            "{backend}: the granted content is disclosed: {text}"
        );
        assert!(
            text.contains("[MEM] container=") && text.contains("label=private"),
            "{backend}: the hit carries the provenance suffix: {text}"
        );
    }
}

#[test]
fn n442_tw_and_vm_agree_on_the_refusal() {
    // Both programs put content under a grant that is revoked inside the
    // program itself — recall then addresses gated memory on both
    // backends and must refuse with the SAME typed stamp.
    for backend in ["tw", "vm"] {
        let scope = format!("memory:{}", unique(&format!("n442-ref-{}", backend)));
        let subject = scope.trim_start_matches("memory:").to_string();
        let src = format!(
            r#"
pattern P(_t: String) -> String {{
  let _ = consent_grant("c", "{scope}", "{subject}", 0.0)
  let h = memory_open("{subject}", "private")
  let _ = memory_put(h, "gated-key", "gated payload")
  let _ = consent_revoke("c", "{scope}")
  return recall("gated-key")
}}
flow Main {{ input: String = "x" -> P -> output }}
"#
        );
        let err = match backend {
            "tw" => run_tw(&src).err(),
            _ => run_vm(&src).err(),
        };
        let err = err.unwrap_or_else(|| panic!("{backend}: the gated recall must refuse"));
        assert!(
            err.contains("MEMORY_RECALL_CONSENT_REQUIRED"),
            "{backend}: the refusal carries the typed stamp: {err}"
        );
    }
}

// ── The static recall surface companion ──────────────────────────────

#[test]
fn n442_static_surface_companion() {
    // A literal non-String query is a broken call site.
    let bad_query = r#"
pattern P() -> String {
  return recall(42.0)
}
flow Main { input: String = "x" -> P -> output }
"#;
    let report = metalogos::audit::audit_program(bad_query).unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check_id == "RECALL_QUERY_INVALID"),
        "audit must carry RECALL_QUERY_INVALID, got: {:?}",
        report
            .findings
            .iter()
            .map(|f| f.check_id)
            .collect::<Vec<_>>()
    );

    // A literal min_confidence outside 0.0..=1.0 is broken too.
    let bad_conf = r#"
pattern P() -> String {
  return recall("notes", 1.5)
}
flow Main { input: String = "x" -> P -> output }
"#;
    let report = metalogos::audit::audit_program(bad_conf).unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check_id == "RECALL_CONFIDENCE_INVALID"),
        "audit must carry RECALL_CONFIDENCE_INVALID, got: {:?}",
        report
            .findings
            .iter()
            .map(|f| f.check_id)
            .collect::<Vec<_>>()
    );

    // A clean call site carries neither.
    let clean = r#"
pattern P() -> String {
  return recall("notes", 0.5)
}
flow Main { input: String = "x" -> P -> output }
"#;
    let report = metalogos::audit::audit_program(clean).unwrap();
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.check_id.starts_with("RECALL_")),
        "a clean recall call site must carry no RECALL_* finding, got: {:?}",
        report
            .findings
            .iter()
            .map(|f| f.check_id)
            .collect::<Vec<_>>()
    );
}

// ── The ledger actions exist as a family ─────────────────────────────

#[test]
fn n442_ledger_family_is_wired() {
    // Self-contained: produce BOTH record kinds here (a granted recall
    // and a gated refusal) so the assertion never depends on test order.
    let subject = unique("n442-family");
    grant_consent(&subject);
    let handle =
        call_builtin("memory_open", &[s(&subject), s("private")]).expect("consented private open");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("family-key"),
            s("family payload"),
            Value::Unit,
        ],
    )
    .expect("put");

    // A granted call → memory.recall.
    call_builtin("recall", &[s("family-key")]).expect("granted recall");
    // A gated call → memory.recall.denied.
    revoke_consent(&subject);
    let refused = call_builtin("recall", &[s("family-key")]);
    assert!(
        refused.is_err(),
        "the revoked address must refuse (the family test's own refusal)"
    );

    let actions = ledger_actions();
    let recall_family: Vec<_> = actions
        .iter()
        .filter(|a| a.starts_with("memory.recall"))
        .collect();
    assert!(
        recall_family.iter().any(|a| a.as_str() == "memory.recall"),
        "memory.recall must be in the ledger family, got: {:?}",
        recall_family
    );
    assert!(
        recall_family
            .iter()
            .any(|a| a.as_str() == "memory.recall.denied"),
        "memory.recall.denied must be in the ledger family, got: {:?}",
        recall_family
    );
}
