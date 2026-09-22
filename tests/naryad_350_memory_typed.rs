// ── Naryad #350 (P1, feature/memory): typed Memory<K> contracts ───────
//
// The registry naryad's test contract:
//   M1  private memory is NOT readable in a public context without
//       redact: the private READ returns Secret — print() refuses it
//       (the existing lattice gate) and redact() is the legal egress;
//       the explicit file export of a private entry REFUSES.
//   M2  derived-from is recorded (the №351 graph raw material) — and
//       validated fail-closed (dangling parents refuse).
//   M3  per-subject isolation: subjects' private containers encrypt
//       under DIFFERENT derived keys — a foreign subject's key cannot
//       decrypt another subject's at-rest blob (introspection gate).
//   M4  the read gives an audit event: the Action-Ledger `memory.*`
//       family records every put/read/keys/provenance/export/open.
//   M5  the consent gate: a private container without an active
//       consent grant refuses (MEMORY_CONSENT_REQUIRED); a public one
//       needs no consent.
//   M6  at-rest: private entries are stored ENCRYPTED (no plaintext at
//       rest), public entries are plaintext.

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

fn open_public(subject: &str) -> Value {
    call_builtin("memory_open", &[s(subject), s("public")]).expect("public open needs no consent")
}

fn open_private(subject: &str) -> Result<Value, String> {
    call_builtin("memory_open", &[s(subject), s("private")])
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

fn handle_id(handle: &Value) -> String {
    match handle {
        Value::Memory(map) => map.get("id").cloned().expect("handle has id"),
        other => panic!("expected Memory handle, got {}", other.type_name()),
    }
}

fn memory_records(container_id: &str) -> Vec<String> {
    all_records()
        .expect("ledger readable")
        .into_iter()
        .filter(|r| r.actor == container_id)
        .map(|r| r.action)
        .collect()
}

// ── M1: private reads are Secret-gated; redact is the only egress ────

#[test]
fn private_read_is_secret_and_redact_is_the_only_egress() {
    let subject = unique("alice-m1");
    grant_consent(&subject);
    let handle = open_private(&subject).expect("consented private open");
    call_builtin("memory_put", &[handle.clone(), s("ssn"), s("123-45-6789")]).expect("put");

    // The read returns Secret — the print/egress gate refuses it.
    let read = call_builtin("memory_read", &[handle.clone(), s("ssn")]).expect("read");
    assert!(
        matches!(read, Value::Secret(_)),
        "private read must be Secret, got {:?}",
        read
    );
    let refused = call_builtin("print", std::slice::from_ref(&read));
    assert!(
        refused.is_err(),
        "print(Secret) must refuse — the public-context read is dead without redact"
    );

    // redact() is the legal path: masked output, printable.
    let masked = call_builtin("redact", &[read, s("all")]).expect("redact accepts Secret");
    let printed = call_builtin("print", &[masked]);
    assert!(printed.is_ok(), "redact output must be printable");

    // The explicit file export of a private entry REFUSES.
    let err = call_builtin(
        "memory_export",
        &[handle.clone(), s("ssn"), s("out/m1.txt")],
    )
    .unwrap_err();
    assert!(
        err.contains("MEMORY_REDACT_REQUIRED"),
        "private export: {}",
        err
    );

    // Public containers read back as plain String (no Secret wrap).
    let pub_handle = open_public(&unique("pub-m1"));
    call_builtin("memory_put", &[pub_handle.clone(), s("note"), s("hello")]).expect("put");
    let pub_read = call_builtin("memory_read", &[pub_handle, s("note")]).expect("read");
    assert_eq!(format!("{:?}", pub_read), r#"String("hello")"#);
}

// ── M2: derived-from recorded + fail-closed parents ──────────────────

#[test]
fn derived_from_recorded_and_validated() {
    let handle = open_public(&unique("m2-deriv"));
    call_builtin("memory_put", &[handle.clone(), s("source"), s("raw facts")]).expect("put root");
    call_builtin(
        "memory_put",
        &[
            handle.clone(),
            s("summary"),
            s("summarized"),
            Value::List(vec![s("source")]),
        ],
    )
    .expect("put derived");

    let prov =
        call_builtin("memory_provenance", &[handle.clone(), s("summary")]).expect("provenance");
    assert_eq!(format!("{:?}", prov), r#"List([String("source")])"#);

    // Dangling parent — loud refusal (the graph integrity for №351).
    let err = call_builtin(
        "memory_put",
        &[
            handle,
            s("bad"),
            s("x"),
            Value::List(vec![s("nonexistent-parent")]),
        ],
    )
    .unwrap_err();
    assert!(
        err.contains("MEMORY_UNKNOWN_PARENT"),
        "dangling parent: {}",
        err
    );
}

// ── M3: per-subject isolation (foreign key cannot decrypt) ───────────

#[test]
fn per_subject_isolation_at_rest() {
    let a = unique("alice-m3");
    let b = unique("bob-m3");
    grant_consent(&a);
    grant_consent(&b);
    let ha = open_private(&a).expect("alice open");
    let hb = open_private(&b).expect("bob open");
    call_builtin(
        "memory_put",
        &[ha.clone(), s("secret-note"), s("alice-private-value")],
    )
    .expect("put");
    call_builtin(
        "memory_put",
        &[hb.clone(), s("secret-note"), s("bob-private-value")],
    )
    .expect("put");

    // Both containers hold the same key NAME — namespaces are disjoint
    // (the subject IS the address).
    let keys_a = call_builtin("memory_keys", std::slice::from_ref(&ha)).expect("keys");
    let keys_b = call_builtin("memory_keys", &[hb]).expect("keys");
    assert_eq!(format!("{:?}", keys_a), r#"List([String("secret-note")])"#);
    assert_eq!(format!("{:?}", keys_b), r#"List([String("secret-note")])"#);

    // The at-rest blobs of alice and bob do NOT decrypt under each
    // other's derived keys (the introspection gate — no key material
    // ever leaves the module).
    assert!(
        metalogos::memory_typed::cross_decrypt_fails(&b, &handle_id(&ha), "secret-note"),
        "bob's derived key must NOT decrypt alice's at-rest blob"
    );
    assert!(
        !metalogos::memory_typed::cross_decrypt_fails(&a, &handle_id(&ha), "secret-note"),
        "alice's own key decrypts her blob"
    );
}

// ── M4: the ledger audit trail (per-container actor filter) ──────────

#[test]
fn every_operation_is_a_ledger_record() {
    let subject = unique("carol-m4");
    grant_consent(&subject);
    let handle = open_private(&subject).expect("open");
    let cid = handle_id(&handle);
    call_builtin("memory_put", &[handle.clone(), s("k"), s("v")]).expect("put");
    let _ = call_builtin("memory_read", &[handle.clone(), s("k")]).expect("read");
    let _ = call_builtin("memory_keys", std::slice::from_ref(&handle)).expect("keys");
    let _ = call_builtin("memory_provenance", &[handle.clone(), s("k")]).expect("prov");
    let _ = call_builtin("memory_export", &[handle, s("k"), s("out/nope.txt")]); // refused — but audited attempt? No: refusals are loud errors, not events.

    let actions = memory_records(&cid);
    assert_eq!(
        actions,
        vec![
            "memory.open",
            "memory.put",
            "memory.read",
            "memory.keys",
            "memory.provenance",
        ],
        "every memory operation must be an Action-Ledger record (in order)"
    );
}

// ── M5: the consent gate on private opens ─────────────────────────────

#[test]
fn private_open_is_consent_gated() {
    let subject = unique("dave-m5");
    let err = open_private(&subject).unwrap_err();
    assert!(
        err.contains("MEMORY_CONSENT_REQUIRED"),
        "no-consent open: {}",
        err
    );

    grant_consent(&subject);
    let handle = open_private(&subject).expect("consented open");
    // Re-open returns the SAME container (the subject is the address).
    let handle2 = open_private(&subject).expect("re-open");
    assert_eq!(
        handle_id(&handle),
        handle_id(&handle2),
        "re-open is the same container"
    );

    // A public container needs NO consent.
    let pub_handle = open_public(&unique("dave-m5-public"));
    assert!(handle_id(&pub_handle).starts_with("mem-"));

    // Unknown words are loud errors (secret/network are not storage K).
    let err = call_builtin("memory_open", &[s("x"), s("secret")]).unwrap_err();
    assert!(
        err.contains("unknown container label"),
        "bad label: {}",
        err
    );
}

// ── M6: at-rest representation (private = encrypted blob) ─────────────

#[test]
fn private_entries_are_encrypted_at_rest() {
    let subject = unique("erin-m6");
    grant_consent(&subject);
    let handle = open_private(&subject).expect("open");
    let hid = handle_id(&handle);
    call_builtin(
        "memory_put",
        &[handle.clone(), s("doc"), s("plaintext-at-rest-check")],
    )
    .expect("put");
    assert!(
        metalogos::memory_typed::entry_is_encrypted(&hid, "doc"),
        "private entries must be stored encrypted at rest"
    );
    // Public: plaintext at rest.
    let pub_handle = open_public(&unique("erin-m6-public"));
    let pub_id = handle_id(&pub_handle);
    call_builtin("memory_put", &[pub_handle, s("doc"), s("public-plaintext")]).expect("put");
    assert!(
        !metalogos::memory_typed::entry_is_encrypted(&pub_id, "doc"),
        "public entries are plaintext at rest"
    );
}

// ── Public export works (the positive side of the M1 gate) ───────────

#[test]
fn public_export_writes_through_the_sandbox() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("chdir");
    std::fs::create_dir_all(dir.path().join("out"))
        .expect("the export parent dir must exist before the sandboxed write");
    let handle = open_public(&unique("fred-export"));
    call_builtin(
        "memory_put",
        &[handle.clone(), s("report"), s("exportable text")],
    )
    .expect("put");
    let out =
        call_builtin("memory_export", &[handle, s("report"), s("out/report.txt")]).expect("export");
    assert_eq!(format!("{:?}", out), r#"String("out/report.txt")"#);
    let written = std::fs::read_to_string(dir.path().join("out/report.txt")).expect("file written");
    assert_eq!(written, "exportable text");
    let _ = std::env::set_current_dir(prev);
}

// ── Fail-closed reads and typing ─────────────────────────────────────

#[test]
fn unknown_keys_and_bad_handles_refuse_loudly() {
    let handle = open_public(&unique("greg-m7"));
    let err = call_builtin("memory_read", &[handle.clone(), s("missing")]).unwrap_err();
    assert!(err.contains("MEMORY_UNKNOWN_KEY"), "missing key: {}", err);
    let err = call_builtin("memory_read", &[s("not-a-memory"), s("k")]).unwrap_err();
    assert!(err.contains("expected Memory"), "bad handle: {}", err);
    let err = call_builtin(
        "memory_put",
        &[handle.clone(), s("k"), Value::Encrypted(vec![1])],
    )
    .unwrap_err();
    assert!(
        err.contains("opaque handles are not memory content"),
        "opaque value: {}",
        err
    );
}
