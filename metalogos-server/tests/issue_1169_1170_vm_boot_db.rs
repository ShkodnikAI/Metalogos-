// ── tests/issue_1169_1170_vm_boot_db.rs ─────────────────────────────────
// The owner's bug reports #1169/#1170 (the office VM-pilot staging, 0.30.0):
//
//   #1170 — `db { url: env("NAME") }` on the VM serve lane resolves the
//   URL INSIDE every route handler (the lazy first-db-access resolution,
//   №758), so the same read goes through the №259 serve-route env gate:
//   without `METALOGOS_ENV_ALLOWLIST` EVERY request 500s with
//   ENV_NOT_PERMITTED while the interpreter — which resolves the url
//   expression at BOOT (init_db_connection, the Process context) — serves
//   the same file + env with 200 and no allowlist.
//
//   #1169 — the same lazy path means the VM executes the schema-DDL
//   bootstrap only when a request first touches the db; on the office's
//   real corpus (30 routes) a fresh DB stayed at 0 tables and the first
//   request died with an unrelated arity 500 instead of an honest
//   bootstrap diagnostic.
//
// The fix (interpreter-parity boot bootstrap): at VM serve boot the URL
// source resolves ONCE in the Process (boot) context — the resolved
// literal replaces `db_url_env` in the Program (no runtime env() reads
// left in handlers) — and the schema DDL replays eagerly on a boot
// connection, so a fresh DB is bootstrapped BEFORE the first request; an
// unset variable is a LOUD BOOT error, not a per-request 500.
//
// Proven here (real axum serve + real sqlite file):
//   T1: env-URL program + a db-touching route, NO allowlist → boot OK,
//       GET /count → 200 (was: 500 ENV_NOT_PERMITTED per request).
//   T2: the same program → the fresh sqlite file carries the schema
//       table immediately after boot (interpreter parity; the office's
//       "0 tables" state is gone).
//   T3: the env variable UNSET → the boot FAILS with a loud message
//       naming the variable (was: a silent OK boot, then 500s).
//   T4: the literal-URL twin of T1 still works end-to-end (no regression
//       on the №758 surface).

#![cfg(feature = "server")]
#![allow(clippy::disallowed_methods)]

use metalogos_server::server::ServeBackend;

fn temp_db_path(tag: &str) -> String {
    let path = std::env::temp_dir().join(format!("issue_1169_{}_{}.db", tag, std::process::id()));
    let _ = std::fs::remove_file(&path);
    format!("sqlite:{}", path.display())
}

fn cleanup_db(url: &str) {
    let path = url.trim_start_matches("sqlite:");
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path));
    let _ = std::fs::remove_file(format!("{}-shm", path));
}

const ENV_DB_PROG: &str = r#"
db { url: env("I1169_TEST_DB_URL") }

schema app {
  table items {
    id: Int primary_key auto_increment,
    name: String
  }
}

mlogserver {
  port: 0
  route "/count" method=GET {
    let n = query_scalar("SELECT COUNT(*) FROM items")
    respond("200", "count=" + to_string(n))
  }
}
"#;

const LITERAL_DB_PROG: &str = r#"
db { url: "I1169_LITERAL_DB_URL_PLACEHOLDER" }

schema app {
  table items {
    id: Int primary_key auto_increment,
    name: String
  }
}

mlogserver {
  port: 0
  route "/count" method=GET {
    let n = query_scalar("SELECT COUNT(*) FROM items")
    respond("200", "count=" + to_string(n))
  }
}
"#;

async fn start_vm_server(source: &str) -> u16 {
    let (port, handle) =
        metalogos_server::server::run_test_server_with_backend(source, ServeBackend::Vm)
            .await
            .expect("test server should start");
    // Detach: the task keeps serving until the process ends; the test's
    // assertions ride the real HTTP stack, then the temp db is removed.
    std::mem::forget(handle);
    port
}

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url).await.expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body readable");
    (status, body)
}

fn schema_table_exists(db_url: &str, table: &str) -> bool {
    let path = db_url.trim_start_matches("sqlite:");
    let conn = rusqlite::Connection::open(path).expect("open the sqlite file");
    let found: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |r| r.get(0),
        )
        .expect("sqlite_master query");
    found > 0
}

// ── T1: the boot-resolved env URL — no allowlist, the route works ──────

#[tokio::test]
#[serial_test::serial]
async fn t1_env_db_url_serves_without_allowlist() {
    let db_url = temp_db_path("t1");
    std::env::set_var("I1169_TEST_DB_URL", &db_url);
    std::env::remove_var("METALOGOS_ENV_ALLOWLIST");

    let port = start_vm_server(ENV_DB_PROG).await;
    let (status, body) = http_get(port, "/count").await;

    assert_eq!(
        status, 200,
        "the db-touching route must serve without METALOGOS_ENV_ALLOWLIST \
         (the URL resolves at boot, never inside the handler); body: {body}"
    );
    assert!(
        body.contains("count=0"),
        "empty table → count=0, got: {body}"
    );

    std::env::remove_var("I1169_TEST_DB_URL");
    cleanup_db(&db_url);
}

// ── T2: the fresh DB is bootstrapped AT BOOT (interpreter parity) ──────

#[tokio::test]
#[serial_test::serial]
async fn t2_fresh_db_is_bootstrapped_at_boot() {
    let db_url = temp_db_path("t2");
    std::env::set_var("I1169_TEST_DB_URL", &db_url);
    std::env::remove_var("METALOGOS_ENV_ALLOWLIST");

    let _port = start_vm_server(ENV_DB_PROG).await;

    assert!(
        schema_table_exists(&db_url, "items"),
        "the schema table must exist immediately after boot \
         (the office's #1169 '0 tables' state is closed)"
    );

    std::env::remove_var("I1169_TEST_DB_URL");
    cleanup_db(&db_url);
}

// ── T3: the unset variable is a LOUD BOOT error, not per-request 500s ──

#[tokio::test]
#[serial_test::serial]
async fn t3_unset_env_var_fails_the_boot_loudly() {
    std::env::remove_var("I1169_TEST_DB_URL");
    std::env::remove_var("METALOGOS_ENV_ALLOWLIST");

    let result =
        metalogos_server::server::run_test_server_with_backend(ENV_DB_PROG, ServeBackend::Vm).await;

    let err = result.expect_err("the boot must fail when the db url env var is unset");
    let text = err.to_string();
    assert!(
        text.contains("I1169_TEST_DB_URL"),
        "the boot error must name the missing variable, got: {text}"
    );
}

// ── T4: the literal-URL twin — no regression on the №758 surface ───────

#[tokio::test]
#[serial_test::serial]
async fn t4_literal_db_url_serves_and_bootstraps() {
    let db_url = temp_db_path("t4");
    let source = LITERAL_DB_PROG.replace("I1169_LITERAL_DB_URL_PLACEHOLDER", &db_url);

    let port = start_vm_server(&source).await;
    let (status, body) = http_get(port, "/count").await;

    assert_eq!(status, 200, "literal db url route must serve; body: {body}");
    assert!(body.contains("count=0"), "got: {body}");
    assert!(
        schema_table_exists(&db_url, "items"),
        "literal-url boot must bootstrap the schema too"
    );

    cleanup_db(&db_url);
}
