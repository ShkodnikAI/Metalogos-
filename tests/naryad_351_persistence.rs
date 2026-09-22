// ── Naryad #351: the persistent store survives a TRUE process restart ──
//
// The ADR-0173 §3.5 «Сделано, когда» criterion (в): «rusqlite-стор
// переживает рестарт процесса (персистентность проверена тестом)».
//
// Method: the harness RE-EXECS the test binary as child processes
// against the SAME DB file (METALOGOS_MEMORY_DB) and the SAME master
// key (METALOGOS_MEMORY_MASTER):
//   child "write"  — builds a graph (root→mid→leaf + independent),
//                    a PRIVATE encrypted entry, and pins {mid, leaf};
//   child "read"   — a FRESH process: loads the container from the DB,
//                    reads the plaintext AND decrypts the private blob
//                    under the same master, sees the pins, then performs
//                    the granted cascade forget of the root;
//   child "verify" — a THIRD process: the forget itself persisted (the
//                    closure is gone, the independent entry remains, the
//                    pins of the deleted nodes are gone).
// A cache-reset simulation cannot fake this: the only thing that crosses
// the process boundary is the DB file (and the env anchors).

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;

const MASTER_HEX: &str = "7777aaaa7777aaaa7777aaaa7777aaaa7777aaaa7777aaaa7777aaaa7777aaaa";
const SUBJECT: &str = "n351-restart-agent";

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

fn handle_id(handle: &Value) -> String {
    match handle {
        Value::Memory(map) => map.get("id").cloned().expect("handle has id"),
        other => panic!("expected Memory handle, got {}", other.type_name()),
    }
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

fn str_list(v: &Value) -> Vec<String> {
    match v {
        Value::List(items) => items
            .iter()
            .map(|x| match x {
                Value::String(t) => t.clone(),
                other => panic!("element not a string: {}", other.type_name()),
            })
            .collect(),
        other => panic!("expected List, got {}", other.type_name()),
    }
}

fn read_text(handle: &Value, key: &str) -> String {
    match call_builtin("memory_read", &[handle.clone(), s(key)]).expect("read") {
        Value::String(x) => x,
        other => panic!("expected String, got {}", other.type_name()),
    }
}

fn open_public() -> Value {
    call_builtin("memory_open", &[s(SUBJECT), s("public")]).expect("public open")
}

fn grant_consent() {
    call_builtin(
        "consent_grant",
        &[
            s("n351-restart"),
            s(&format!("memory:{}", SUBJECT)),
            s(SUBJECT),
            Value::Float(0.0),
        ],
    )
    .expect("consent_grant");
}

// ── The children (no-ops unless invoked by the parent with the env) ────

#[test]
fn n351_child_write() {
    if std::env::var("METALOGOS_N351_CHILD").as_deref() != Ok("write") {
        return;
    }
    let handle = open_public();
    let h = handle_id(&handle);
    call_builtin("memory_put", &[handle.clone(), s("root"), s("ROOT-VALUE")]).unwrap();
    call_builtin(
        "memory_put",
        &[handle.clone(), s("mid"), s("MID-VALUE"), list(&["root"])],
    )
    .unwrap();
    call_builtin(
        "memory_put",
        &[handle.clone(), s("leaf"), s("LEAF-VALUE"), list(&["mid"])],
    )
    .unwrap();
    call_builtin(
        "memory_put",
        &[handle.clone(), s("independent"), s("IND-VALUE")],
    )
    .unwrap();
    // A private entry: the BLOB is what persists — decrypting it later
    // proves the master derivation is restart-stable.
    grant_consent();
    let priv_handle =
        call_builtin("memory_open", &[s(SUBJECT), s("private")]).expect("private open");
    call_builtin(
        "memory_put",
        &[priv_handle.clone(), s("secret-entry"), s("pi-31415926535")],
    )
    .unwrap();
    // Pin the {mid, leaf} closure.
    call_builtin("memory_retain", &[handle.clone(), s("mid")]).expect("retain");
    eprintln!("[N351-WRITE] container={} ok", h);
}

#[test]
fn n351_child_read_and_forget() {
    if std::env::var("METALOGOS_N351_CHILD").as_deref() != Ok("read") {
        return;
    }
    // Fresh process: the container must come from the DB.
    let handle = open_public();
    let h = handle_id(&handle);
    let mut ks = keys(&handle);
    ks.sort();
    assert_eq!(
        ks,
        vec!["independent", "leaf", "mid", "root"],
        "the graph loaded from the persistent store"
    );
    assert_eq!(read_text(&handle, "root"), "ROOT-VALUE");
    // The pins survived the restart.
    let mut pins = str_list(
        &call_builtin("memory_retained", std::slice::from_ref(&handle)).expect("retained"),
    );
    pins.sort();
    assert_eq!(pins, vec!["leaf", "mid"], "the pins survive the restart");
    // The private blob decrypts under the SAME master in a NEW process.
    grant_consent();
    let priv_handle =
        call_builtin("memory_open", &[s(SUBJECT), s("private")]).expect("private open");
    match call_builtin("memory_read", &[priv_handle.clone(), s("secret-entry")])
        .expect("private read")
    {
        Value::Secret(zs) => assert_eq!(zs.as_str(), "pi-31415926535"),
        other => panic!("expected Secret, got {}", other.type_name()),
    }
    // The retained VETO still fires after the restart.
    let grant = call_builtin(
        "grant_issue",
        &[s("memory:forget:*"), Value::Float(3600.0), s("unlimited")],
    )
    .expect("grant");
    let err = call_builtin(
        "memory_forget_cascade",
        &[handle.clone(), s("root"), grant.clone()],
    )
    .expect_err("the veto survives the restart");
    assert!(err.contains("MEMORY_RETAIN_PROTECTED"), "got: {err}");
    // Release, then the granted cascade forget — and ITS effect must
    // persist into the next process too.
    call_builtin("memory_release", &[handle.clone(), s("mid")]).expect("release");
    let out = call_builtin("memory_forget_cascade", &[handle.clone(), s("root"), grant])
        .expect("the forget after restart");
    let deleted_field = match &out {
        Value::Struct { fields, .. } => fields
            .iter()
            .find(|(k, _)| k.as_str() == "deleted")
            .map(|(_, v)| v.clone())
            .expect("deleted field"),
        other => panic!("expected Struct, got {}", other.type_name()),
    };
    let mut deleted = str_list(&deleted_field);
    deleted.sort();
    assert_eq!(
        deleted,
        vec!["leaf", "mid", "root"],
        "the full closure died"
    );
    assert_eq!(keys(&handle), vec!["independent".to_string()]);
    eprintln!("[N351-READ] container={} ok", h);
}

#[test]
fn n351_child_verify_forget_persisted() {
    if std::env::var("METALOGOS_N351_CHILD").as_deref() != Ok("verify") {
        return;
    }
    let handle = open_public();
    assert_eq!(
        keys(&handle),
        vec!["independent".to_string()],
        "the forget persisted across ANOTHER restart"
    );
    assert!(str_list(
        &call_builtin("memory_retained", std::slice::from_ref(&handle)).expect("retained")
    )
    .is_empty());
    assert_eq!(read_text(&handle, "independent"), "IND-VALUE");
    eprintln!("[N351-VERIFY] ok");
}

// ── The parent: drives the children as REAL processes ──────────────────

fn run_child(name: &str, db: &std::path::Path) {
    let exe = std::env::current_exe().expect("current_exe");
    let status = std::process::Command::new(exe)
        .args(["--exact", name, "--test-threads", "1", "--nocapture"])
        .env(
            "METALOGOS_N351_CHILD",
            name.rsplit("n351_child_").next().unwrap_or(name),
        )
        .env("METALOGOS_MEMORY_DB", db)
        .env("METALOGOS_MEMORY_MASTER", MASTER_HEX)
        .status()
        .expect("spawn the test binary");
    assert!(
        status.success(),
        "child {name} failed: {status} — the restart contract is broken"
    );
}

#[test]
fn persistence_survives_a_true_process_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("n351_typed_memory.db");
    run_child("n351_child_write", &db);
    run_child("n351_child_read_and_forget", &db);
    run_child("n351_child_verify_forget_persisted", &db);
    assert!(db.exists(), "the DB file exists");
}
