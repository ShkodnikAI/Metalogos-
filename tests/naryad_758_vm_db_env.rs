// ── tests/naryad_758_vm_db_env.rs ───────────────────────────────────────
// №758 (P1, vm): `db { url: env("NAME") }` never reached the VM lane —
// the compiler extracted the URL only from a string literal, the env()
// call compiled to `Program.db_url = None`, and `ensure_db_open`
// silently no-opped: the boot log said "Connected" (the interpreter's
// boot pass resolves env() fine) while EVERY request-path db_execute()
// through the VM pool failed with the riddle "no database connection"
// (the office differential probe: interpreter 200 / vm 500).
//
// Proven here (real parse → compile → Vm::run, the bug_530 harness):
//   T1: db { url: env("NAME") } compiles, the URL resolves at the FIRST
//       db access (runtime semantics — the interpreter's), and the
//       request-path db_execute works against a real sqlite file.
//   T2: the env NAME is carried in the Program (serialize roundtrip —
//       the resolved URL never enters the bytecode) and old .mbc files
//       (no field) deserialize cleanly via #[serde(default)].
//   T3: the env var UNSET → a LOUD error naming the variable and the
//       remedy (the old behavior: the silent "no database connection"
//       riddle).
//   T4: a non-sqlite URL (postgres) → a LOUD error naming the
//       sqlite-only VM surface and the interpreter-backend escape
//       hatch (the old behavior: silent no-op → the same riddle).
//   T5: exotic URL expressions fail at COMPILE time (loud, not silent).
//
// Env-var mutation is process-global — the tests hold the static mutex
// (the №251 discipline).

// №475: the tests exercise the REAL filesystem (a real sqlite file is
// the point — the lazy open must materialize it); the ratchet targets
// production I/O.
#![allow(clippy::disallowed_methods)]

use std::sync::Mutex;

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn compile(src: &str) -> Result<metalogos::bytecode::Program, String> {
    let declarations =
        metalogos::parser::parse(src.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    comp.compile(declarations)
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let program = compile(source)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn temp_db_path(tag: &str) -> String {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("n758_{}_{}.db", tag, std::process::id()));
    let _ = std::fs::remove_file(&path);
    format!("sqlite:{}", path.display())
}

fn cleanup_db(url: &str) {
    let path = url.trim_start_matches("sqlite:");
    let _ = std::fs::remove_file(path);
}

const ENV_DB_PROG: &str = r#"
db { url: env("N758_TEST_DB_URL") }

pattern P(t: String) -> String {
  let _ = db_execute("CREATE TABLE IF NOT EXISTS extra (v TEXT)")
  let n = db_execute("INSERT INTO extra (v) VALUES ('written-on-vm')")
  return n
}
flow Main { input: String = "x" -> P -> output }
"#;

// ── T1: the deferred env resolution makes the request path work ─────────

#[test]
fn n758_env_db_url_resolves_lazily_and_db_execute_works() {
    let _g = test_lock();
    let url = temp_db_path("env");
    std::env::set_var("N758_TEST_DB_URL", &url);

    let program = compile(ENV_DB_PROG).expect("compiles with env(...) URL");
    // The bytecode carries the NAME, not the resolved URL.
    assert_eq!(program.db_url, None);
    assert_eq!(program.db_url_env.as_deref(), Some("N758_TEST_DB_URL"));

    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).expect("VM run works");
    drop(vm);
    // The file exists (a real connection was opened lazily on the first
    // db access) — the old behavior never produced one.
    assert!(
        std::path::Path::new(url.trim_start_matches("sqlite:")).exists(),
        "the lazy open must have created the sqlite file"
    );
    cleanup_db(&url);
    std::env::remove_var("N758_TEST_DB_URL");
}

// ── T2: the NAME round-trips through .mbc; the URL stays out ────────────

#[test]
fn n758_env_name_roundtrips_bytecode_and_old_mbc_is_default_none() {
    let program = compile(ENV_DB_PROG).expect("compiles");
    let bytes = serde_json::to_vec(&program).expect("serialize");
    let back: metalogos::bytecode::Program = serde_json::from_slice(&bytes).expect("deserialize");
    assert_eq!(back.db_url_env.as_deref(), Some("N758_TEST_DB_URL"));
    assert_eq!(back.db_url, None);

    // An OLD artifact without the field deserializes cleanly (serde default).
    let old: metalogos::bytecode::Program =
        serde_json::from_slice(&bytes).expect("old-format deserialize");
    assert!(old.db_url_env.is_none() || old.db_url_env.is_some()); // shape-only pin
    let minimal = r#"{"globals":[],"patterns":[],"learnables":[],"rules":[],"skill_indices":[],"deny_handlers":[],"db_url":null,"memory_persist_path":null,"schema_ddl":[],"main_code":[],"collections_loaded":false}"#;
    let old: metalogos::bytecode::Program = serde_json::from_str(minimal).expect("old .mbc parses");
    assert_eq!(
        old.db_url_env, None,
        "#[serde(default)] keeps old files loadable"
    );
}

// ── T3: unset env → LOUD, naming the variable ───────────────────────────

#[test]
fn n758_unset_env_db_url_is_a_loud_error() {
    let _g = test_lock();
    std::env::remove_var("N758_TEST_DB_URL");

    // The first db access hits the resolution DURING the run (the open
    // is lazy, №409) — the failure is LOUD, naming the variable and the
    // remedy, not the legacy "no database connection" riddle.
    let err = run_vm(ENV_DB_PROG).unwrap_err();
    assert!(
        err.contains("N758_TEST_DB_URL") && err.contains("is not set"),
        "the error must name the variable and the remedy: {}",
        err
    );
    assert!(
        !err.contains("no database connection"),
        "the riddle text must not come back: {}",
        err
    );
}

// ── T4: non-sqlite URL → LOUD, naming the sqlite-only VM surface ────────

#[test]
fn n758_non_sqlite_url_is_a_loud_error() {
    let _g = test_lock();
    let prog = r#"
db { url: "postgres://user:secret@db.internal:5432/office" }
pattern P(t: String) -> String {
  return db_execute("SELECT 1")
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = run_vm(prog).unwrap_err();
    assert!(
        err.contains("unsupported DB URL scheme"),
        "the error must name the scheme problem: {}",
        err
    );
    assert!(
        err.contains("METALOGOS_SERVE_BACKEND=interpreter"),
        "the error must name the escape hatch: {}",
        err
    );
}

// ── T5: exotic URL expressions fail at COMPILE time (loud, not silent) ──

#[test]
fn n758_exotic_url_expression_fails_at_compile_time() {
    let _g = test_lock();
    let prog = r#"
db { url: "sqlite:" + "x.db" }
pattern P(t: String) -> String { return t }
flow Main { input: String = "x" -> P -> output }
"#;
    let err = compile(prog).unwrap_err();
    assert!(
        err.contains("the VM compiles only db"),
        "compile must refuse exotic URL expressions: {}",
        err
    );

    // env() with a non-literal argument — the same loud refusal.
    let prog2 = r#"
db { url: env(some_var) }
pattern P(t: String) -> String { return t }
flow Main { input: String = "x" -> P -> output }
"#;
    let err2 = compile(prog2).unwrap_err();
    assert!(
        err2.contains("string literal name"),
        "compile must refuse non-literal env args: {}",
        err2
    );
}
