// ── Naryad #381 (issue #467): Stage 4 corpus contract + serve parity ──
//
// ADR-0141 §D5: the Stage 4 benchmark corpus (benches/fixtures/
// production_workload.mlog) is the representative production-class
// workload the backend-flip decision is made on. This file pins its
// contract so CI keeps it honest:
//
//   1. corpus contract: >= 2000 lines (production-class), sanitize
//      gate 0 hits (no secrets / credentials / endpoints);
//   2. semantic gate: the corpus passes the repo's OWN Category-A
//      gate (check_program errors empty — LLM-derived decisions go
//      through escape_html() per №327);
//   3. VM compiles the WHOLE corpus (any future language construct in
//      the corpus that the VM cannot compile fails here, loudly);
//   4. serve parity: ALL benchmark routes return identical
//      (status, body) on both backends over real HTTP;
//   5. regression: VM query()/db_execute()/query_scalar() bind the
//      params list TYPED (№381 fix — the VM previously dropped the
//      params of query() entirely, caught by this corpus);
//   6. regression: in-memory db_conn survives the server-startup
//      merge chain (№381 fix — every merge after the db {} block
//      clobbered the shared connection; routes then failed with
//      "no database connection").
//
// Verify: cargo test --test naryad_381_stage4_corpus

#![cfg(feature = "server")]

use metalogos::server::{run_test_server_with_backend, ServeBackend};
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::Mutex;

const CORPUS_REL: &str = "benches/fixtures/production_workload.mlog";
const PLAN_REL: &str = "benches/fixtures/stage4_routes.json";

/// Mirrors the sanitize gate of the corpus generator and the bench
/// harness — one SSOT contract across all three consumers.
const BANNED: [&str; 11] = [
    "sk-",
    "api_key",
    "apikey",
    "bearer ",
    "private key",
    "akia",
    "ghp_",
    "xox",
    "secret:",
    "token:",
    "authorization:",
];

/// Serializes the HTTP servers in this binary (each test starts one).
/// tokio::sync::Mutex — the guard is held across .await points.
static SERVER_LOCK: Mutex<()> = Mutex::const_new(());

fn manifest_dir() -> PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("cwd"))
}

fn read_corpus() -> String {
    let path = manifest_dir().join(CORPUS_REL);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read corpus {}: {}", path.display(), e))
}

fn read_plan() -> Value {
    let path = manifest_dir().join(PLAN_REL);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read plan {}: {}", path.display(), e));
    serde_json::from_str(&text).expect("plan JSON")
}

fn sanitize_hits(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    BANNED
        .iter()
        .copied()
        .filter(|b| lower.contains(b))
        .collect()
}

// ── 1. Corpus contract ────────────────────────────────────────────────

#[test]
fn n381_corpus_lines_and_sanitize() {
    let corpus = read_corpus();
    let lines = corpus.lines().count();
    assert!(
        lines >= 2000,
        "ADR-0141 §D5: corpus must be >= 2000 lines (production-class), got {}",
        lines
    );
    let hits = sanitize_hits(&corpus);
    assert!(
        hits.is_empty(),
        "sanitize gate: banned patterns present in the corpus: {:?}",
        hits
    );
}

#[test]
fn n381_plan_contract() {
    let plan = read_plan();
    let routes = plan["routes"].as_array().expect("plan.routes");
    assert!(
        routes.len() >= 10,
        "the request plan must cover the full benchmark surface (>= 10 routes), got {}",
        routes.len()
    );
    let mut names = Vec::new();
    for r in routes {
        let name = r["name"].as_str().expect("route.name");
        assert!(r["path"].is_string(), "route {} missing path", name);
        assert!(r["method"].is_string(), "route {} missing method", name);
        names.push(name.to_string());
    }
    let unique = names.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        unique.len(),
        names.len(),
        "route names must be unique: {:?}",
        names
    );
}

// ── 2. Semantic gate + 3. VM compiles the whole corpus ────────────────

#[test]
fn n381_corpus_passes_category_a_and_vm_compile() {
    let corpus = read_corpus();
    let decls = metalogos::parser::parse(&corpus).expect("corpus must parse");
    let sem = metalogos::semantic::check_program(&decls);
    let msgs: Vec<String> = sem.errors.iter().map(|e| e.message.clone()).collect();
    assert!(
        msgs.is_empty(),
        "the corpus must pass the repo's own semantic/Category-A gate ({} errors): {:?}",
        msgs.len(),
        msgs.first()
    );
    // Any future corpus construct the VM cannot compile fails HERE —
    // loud, in CI, before the benchmark ever runs.
    let program =
        metalogos::compile_program(&corpus).expect("the VM must compile the whole Stage 4 corpus");
    assert!(
        !format!("{:?}", std::hint::black_box(&program)).is_empty(),
        "compiled program must be material"
    );
}

// ── 4. Serve parity over HTTP on both backends ────────────────────────

async fn run_all_routes(backend: ServeBackend) -> Vec<(String, u16, String)> {
    let corpus = read_corpus();
    let plan = read_plan();
    let routes = plan["routes"].as_array().expect("plan.routes").clone();

    let (port, handle) = run_test_server_with_backend(&corpus, backend)
        .await
        .expect("test server must start");

    let client = reqwest::Client::new();
    let mut results = Vec::new();
    for r in &routes {
        let mut url = format!(
            "http://127.0.0.1:{}{}",
            port,
            r["path"].as_str().unwrap_or("/")
        );
        if let Some(q) = r["query"].as_object() {
            let qs: Vec<String> = q
                .iter()
                .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or_default()))
                .collect();
            if !qs.is_empty() {
                url.push('?');
                url.push_str(&qs.join("&"));
            }
        }
        let resp = match r["method"].as_str() {
            Some("POST") => client
                .post(&url)
                .json(r["body"].as_object().unwrap_or(&serde_json::Map::new()))
                .send()
                .await
                .unwrap_or_else(|e| panic!("{}: request failed: {}", r["name"], e)),
            _ => client
                .get(&url)
                .send()
                .await
                .unwrap_or_else(|e| panic!("{}: request failed: {}", r["name"], e)),
        };
        let status = resp.status().as_u16();
        let body = resp.text().await.expect("response body");
        results.push((r["name"].as_str().unwrap_or("?").to_string(), status, body));
    }
    handle.abort();
    results
}

#[tokio::test]
async fn n381_route_parity_interpreter_vs_vm() {
    let _guard = SERVER_LOCK.lock().await;

    let interp = run_all_routes(ServeBackend::Interpreter).await;
    let vm = run_all_routes(ServeBackend::Vm).await;

    assert_eq!(
        interp.len(),
        vm.len(),
        "both backends must serve all routes"
    );
    for ((name_i, status_i, body_i), (name_v, status_v, body_v)) in interp.iter().zip(vm.iter()) {
        assert_eq!(name_i, name_v, "route order must match the plan");
        assert_eq!(
            status_i, status_v,
            "route {}: status mismatch (TW vs VM)",
            name_i
        );
        assert_eq!(
            *status_i, 200u16,
            "route {}: handler error (loud parity failure) — body: {}",
            name_i, body_i
        );
        assert_eq!(
            body_i, body_v,
            "route {}: body mismatch between backends\nTW: {}\nVM: {}",
            name_i, body_i, body_v
        );
    }
    // The two №381 regressions are covered transitively by the parity
    // asserts above: without the db_conn merge fix the INTERPRETER
    // /db/list handler 500s ("no database connection"); without the VM
    // params fix the VM /db/orders handler 500s ("Wrong number of
    // parameters"). The focused unit-level pins follow below.
}

// ── 5. Focused regression: VM binds query/db_execute params typed ─────

fn tw_and_vm_output(source: &str) -> (String, String) {
    let tw = metalogos::run_program(source)
        .expect("TW run must succeed")
        .unwrap_or_default();
    let program = metalogos::compile_program(source).expect("VM compile must succeed");
    let vm = metalogos::run_bytecode(program)
        .expect("VM run must succeed")
        .unwrap_or_default();
    (tw.trim_end().to_string(), vm.trim_end().to_string())
}

#[test]
fn n381_vm_query_binds_params_typed() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern T(_x: String) -> String {
  query("CREATE TABLE IF NOT EXISTS items (v REAL, s TEXT, b INTEGER)", [])
  query("INSERT INTO items (v, s, b) VALUES (?, ?, ?)", [2.5, "hello", true])
  let rows = query("SELECT v, s FROM items WHERE b = ? AND v > ?", [1, 1.0])
  let row = get(rows, 0.0)
  return to_string(row.v) + "/" + row.s
}
flow Main {
  input: String = "go" -> T -> output
}
"#;
    let (tw, vm) = tw_and_vm_output(src);
    assert_eq!(tw, "2.5/hello", "TW parameterized query");
    assert_eq!(vm, tw, "VM must bind params typed — parity with TW");
}

#[test]
fn n381_vm_db_execute_and_query_scalar_params() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern T(_x: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS scores (v REAL, tag TEXT)", [])
  db_execute("INSERT INTO scores (v, tag) VALUES (?, ?)", [7.5, "alpha"])
  db_execute("INSERT INTO scores (v, tag) VALUES (?, ?)", [3.0, "beta"])
  let total = query_scalar("SELECT SUM(v) FROM scores WHERE tag != ?", ["none"])
  return to_string(total)
}
flow Main {
  input: String = "go" -> T -> output
}
"#;
    let (tw, vm) = tw_and_vm_output(src);
    assert_eq!(tw, "10.5", "TW db_execute + query_scalar with params");
    assert_eq!(vm, tw, "VM db_execute/query_scalar params parity");
}
