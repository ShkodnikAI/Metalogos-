// ── tests/naryad_535_blocking_checks.rs ─────────────────────────────────
// №535 (P1, process/testing; the consolidated audit 30.09 N-1 class, the
// gate-0.28 checklist item 3, ADR-0179): the machine-readable record of
// WHO BLOCKS EXECUTION — scripts/ci/blocking_checks.tsv — is validated
// against the ACTUAL behavior. Every cell of the table is either:
//   - pinned HERE behaviorally (the n535_* probes drive the REAL `mlog`
//     binary through CARGO_BIN_EXE — exit codes and stderr are the
//     verdicts), or
//   - pinned by the existing test named in the table's test_id column
//     (n98_*, n523_* — the sync script greps them), or
//   - an honest `absent` row (warns-dropped-on-run, the empty exemption
//     list, JIT) whose absence is the record itself. mcp-serve carried the
//     real read/parse/audit/semantic cells since №557 — no absent row
//     there anymore (gh#918).
// A behavior change without a table update (or a table edit without the
// behavior) breaks this file or the blocking-checks-sync CI job — the
// N-1 class ("фильтр по подстроке"/"молчаливый пропуск") re-opens only
// THROUGH a recorded, gated decision now.
#![allow(clippy::disallowed_methods)]

use std::process::Command;

// №567: the test moved with the CLI bin; the repo-root table is one more level up.
const TABLE: &str = include_str!("../../scripts/ci/blocking_checks.tsv");
const SELF: &str = include_str!("naryad_535_blocking_checks.rs");

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mlog")
}

fn rows() -> Vec<Vec<String>> {
    TABLE
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .filter(|l| !l.starts_with("command|phase|check|"))
        .map(|l| l.split('|').map(|s| s.trim().to_string()).collect())
        .collect()
}

fn write_program(name: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("n535_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    path
}

// ── the table's structural integrity ────────────────────────────────────

#[test]
fn n535_table_covers_all_commands() {
    let rows = rows();
    assert!(
        rows.len() >= 25,
        "the table carries at least 25 cells, got {}",
        rows.len()
    );
    for cmd in ["run", "check", "compile", "serve", "mcp-serve"] {
        assert!(
            rows.iter().any(|r| r[0] == cmd),
            "the command `{cmd}` must have rows"
        );
    }
    // The №535 boundary: JIT is recorded as absent. №557: mcp-serve is NOT
    // — the MCP surface carries real cells, the matrix must not regress to
    // an absent row there.
    assert!(rows.iter().any(|r| r[0] == "jit" && r[3] == "absent"));
    assert!(
        !rows.iter().any(|r| r[0] == "mcp-serve" && r[3] == "absent"),
        "mcp-serve must not regress to an absent row (№557: the gate exists)"
    );
    // The verdict domain.
    for r in &rows {
        assert!(
            matches!(r[3].as_str(), "blocks" | "warns" | "absent"),
            "bogus verdict in: {:?}",
            r
        );
    }
    // Every n535_* test_id named by the table exists in THIS file — the
    // self-binding (the python sync greps the rest).
    for r in &rows {
        let id = &r[4];
        if !id.is_empty() && id.starts_with("n535_") {
            assert!(
                SELF.contains(id.as_str()),
                "the table names {id} but this file has no such test"
            );
        }
    }
}

// ── run: the behavioral pins ────────────────────────────────────────────

#[test]
fn n535_run_missing_file_blocks() {
    let out = Command::new(bin())
        .args(["run", "/nonexistent/n535.mlog"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "an unreadable file must block");
}

#[test]
fn n535_run_parse_error_blocks() {
    let p = write_program("parse_err.mlog", "flow broken( { emit 1 }\n");
    let out = Command::new(bin())
        .args(["run", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a parse error must block run");
    assert!(String::from_utf8_lossy(&out.stderr).contains("parse error"));
}

#[test]
fn n535_run_missing_import_blocks() {
    let _dir = std::env::temp_dir().join(format!("n535_{}", std::process::id()));
    let p = write_program(
        "imp_missing.mlog",
        "import std/no_such_module_xyz\nentity a: String = \"x\"\nflow Main { input: String = a -> output }\n",
    );
    let out = Command::new(bin())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["run", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a missing import must block run");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("Naryad #523"),
        "the refusal carries the №523 stamp: {err}"
    );
}

#[test]
fn n535_run_runtime_error_blocks() {
    let p = write_program(
        "runtime_err.mlog",
        "entity u: String = is_admn(user)\nflow Main { input: String = u -> output }\n",
    );
    let out = Command::new(bin())
        .args(["run", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a runtime error must block run");
    assert!(String::from_utf8_lossy(&out.stderr).contains("UNDEFINED_VARIABLE"));
}

// ── compile: the behavioral pins ────────────────────────────────────────

#[test]
fn n535_compile_missing_file_blocks() {
    let out = Command::new(bin())
        .args(["compile", "/nonexistent/n535.mlog"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn n535_compile_parse_error_blocks() {
    let p = write_program("c_parse_err.mlog", "flow broken( { emit 1 }\n");
    let out = Command::new(bin())
        .args(["compile", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn n535_compile_cat_a_blocks() {
    // query(variable) is SQL_DYNAMIC — a Category A violation blocks the
    // compile path too (the same audit_category_a gate as run, №98).
    let p = write_program(
        "c_cat_a.mlog",
        "pattern BadQuery(table: String) -> String {\n  let result = query(table)\n  return result\n}\n",
    );
    let out = Command::new(bin())
        .args(["compile", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "the Cat A violation must block compile"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("Category A") || err.contains("SQL_DYNAMIC"),
        "got: {err}"
    );
}

#[test]
fn n535_compile_import_warns() {
    // A resolvable import: the warning prints (by design) and the compile
    // PROCEEDS — the warns verdict.
    let p = write_program(
        "c_imp_ok.mlog",
        "import std/string\nentity a: String = trim(\"  x  \")\nflow Main { input: String = a -> output }\n",
    );
    // №567: the std/ module corpus lives at the repo root (the manifest
    // dir's parent) — the import resolves from the child's CWD.
    let std_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let out = Command::new(bin())
        .current_dir(std_root)
        .args(["compile", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "a resolvable import must NOT block compile"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("warning: import"),
        "the import warning must print: {err}"
    );
}

#[test]
fn n535_compile_missing_import_blocks() {
    let p = write_program(
        "c_imp_missing.mlog",
        "import std/no_such_module_xyz\nentity a: String = \"x\"\nflow Main { input: String = a -> output }\n",
    );
    let out = Command::new(bin())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["compile", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a missing import must block compile");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("warning: import"),
        "the warning precedes the loud error"
    );
}

// ── check: the behavioral pins ──────────────────────────────────────────

#[test]
fn n535_check_missing_file_blocks() {
    let out = Command::new(bin())
        .args(["check", "/nonexistent/n535.mlog"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn n535_check_parse_error_blocks() {
    let p = write_program("k_parse_err.mlog", "flow broken( { emit 1 }\n");
    let out = Command::new(bin())
        .args(["check", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn n535_check_semantic_error_blocks() {
    // The №523 exploit shape: an undefined function in a condition —
    // `mlog check` reports the error and exits 1.
    let p = write_program(
        "k_sem.mlog",
        "pattern AdminCheck(u: String) -> String {\n  if is_admn(u) {\n    return \"ACCESS GRANTED\"\n  }\n  return \"denied\"\n}\nentity who: String = \"bob\"\nflow Main { input: String = who -> AdminCheck -> output }\n",
    );
    let out = Command::new(bin())
        .args(["check", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success(), "the semantic error must fail check");
    let out_str = String::from_utf8_lossy(&out.stdout);
    assert!(
        out_str.contains("is_admn"),
        "the finding names the function: {out_str}"
    );
}

#[test]
fn n535_check_warning_warns() {
    // A warning-only program: the warning PRINTS and the exit stays 0 —
    // warn, never block.
    let p = write_program(
        "k_warn.mlog",
        "entity thing: UndeclaredType = \"x\"\nflow Main { input: String = \"ok\" -> output }\n",
    );
    let out = Command::new(bin())
        .args(["check", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "a warning must NOT block check");
    let out_str = String::from_utf8_lossy(&out.stdout);
    assert!(
        out_str.contains("warning"),
        "the warning must be reported: {out_str}"
    );
}

// ── serve: the behavioral pins (the failing-startup cases only) ─────────

#[test]
fn n535_serve_missing_file_blocks() {
    let out = Command::new(bin())
        .args(["serve", "/nonexistent/n535.mlog"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn n535_serve_parse_error_blocks() {
    let p = write_program("s_parse_err.mlog", "flow broken( { emit 1 }\n");
    let out = Command::new(bin())
        .args(["serve", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a parse error must refuse the serve startup"
    );
}

#[test]
fn n535_serve_cat_a_blocks() {
    let p = write_program(
        "s_cat_a.mlog",
        "pattern BadQuery(table: String) -> String {\n  let result = query(table)\n  return result\n}\n",
    );
    let out = Command::new(bin())
        .args(["serve", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "the Cat A violation must refuse the serve startup"
    );
}
