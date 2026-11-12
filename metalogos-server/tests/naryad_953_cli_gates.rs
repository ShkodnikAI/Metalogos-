// ── №953 (issue #1920, the audit 12.11): the CLI gates on every input ──
//
// The audit PoC set 5_check_root_and_test (commit fd384d3) + the input→gate
// inventory: every execution input of the CLI sits behind the Category-A
// gates, and the analysis inputs execute NOTHING. These tests spawn the
// REAL binary (CARGO_BIN_EXE_mlog) — the same surface the audit attacked.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mlog")
}

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("n953cli_{}_{}", tag, std::process::id()));
    fs::create_dir_all(&dir).expect("tmp dir");
    dir
}

fn run(dir: &PathBuf, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .current_dir(dir)
        .env("HOME_SECRET_PROBE", "TOPSECRET123")
        .output()
        .expect("spawn mlog");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

const LEAK_ROOT: &str = r#"
pattern Leak(x: String) -> String {
  let s = env("HOME_SECRET_PROBE")
  let _ = write_file("LEAKED_CLI.txt", "secret=" + s)
  print("leak: " + s)
  return "done"
}
flow Main { input: String = "x" -> Leak -> output }
"#;

const CLEAN_LEAF: &str = "pattern Leaf(a: String) -> String { return a }";

/// The audit C-2 probe (leaf.mlog --root root2.mlog): NO execution, NO
/// side-effect file, NO secret on stdout — and the leaking root is
/// honestly REFUSED by the static analysis.
#[test]
fn n953_cli_check_root_executes_nothing() {
    let dir = tmp_dir("c2");
    fs::write(dir.join("leaf.mlog"), CLEAN_LEAF).expect("leaf");
    fs::write(dir.join("root.mlog"), LEAK_ROOT).expect("root");
    let (code, stdout, stderr) = run(&dir, &["check", "leaf.mlog", "--root", "root.mlog"]);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        !combined.contains("leak: TOPSECRET"),
        "the root's print fired — check --root executed the root"
    );
    assert!(
        !combined.contains("TOPSECRET123"),
        "the secret value surfaced in the check output"
    );
    assert!(
        !dir.join("LEAKED_CLI.txt").exists(),
        "the root's write_file fired — check --root executed the root"
    );
    assert_eq!(
        code, 1,
        "the leaking root must be refused (exit 1) — the static analysis sees it now"
    );
    assert!(
        combined.contains("SECRET_LEAK"),
        "the refusal must name SECRET_LEAK, got: {}",
        combined
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The clean pair passes with exit 0 (the contract of the command holds).
#[test]
fn n953_cli_check_root_clean_pair_passes() {
    let dir = tmp_dir("clean");
    fs::write(dir.join("leaf.mlog"), CLEAN_LEAF).expect("leaf");
    fs::write(
        dir.join("root.mlog"),
        "pattern Helper(a: String) -> String { return upper(a) }\n",
    )
    .expect("root");
    let (code, stdout, _stderr) = run(&dir, &["check", "leaf.mlog", "--root", "root.mlog"]);
    assert_eq!(code, 0, "the clean pair must pass, got: {}", stdout);
    assert!(
        stdout.contains("OK"),
        "expected the OK verdict, got: {}",
        stdout
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The audit H-1 probe (t.mlog): `mlog test` refuses BEFORE execution —
/// no `1/1 tests passed`, no side-effect file.
#[test]
fn n953_cli_mlog_test_refuses_the_leak() {
    let dir = tmp_dir("h1");
    fs::write(
        dir.join("t.mlog"),
        format!(
            "{}\ntest \"leak in test\" {{\n  assert_eq(Leak(\"x\"), \"done\")\n}}",
            LEAK_ROOT
        ),
    )
    .expect("t.mlog");
    let (code, stdout, stderr) = run(&dir, &["test", "t.mlog"]);
    let combined = format!("{}{}", stdout, stderr);
    assert_ne!(code, 0, "the gated mlog test must refuse: {}", combined);
    assert!(
        combined.contains("Category A security invariant violated"),
        "the refusal must carry the Category-A gate, got: {}",
        combined
    );
    assert!(
        !combined.contains("1/1 tests passed"),
        "the tests must NOT run behind the refusal"
    );
    assert!(
        !dir.join("LEAKED_CLI.txt").exists(),
        "the leak executed despite the gate"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The explicit opt-out is LOUD: the banner fires, the tests run.
#[test]
fn n953_cli_mlog_test_allow_unchecked_is_loud() {
    let dir = tmp_dir("unchecked");
    fs::write(
        dir.join("t.mlog"),
        format!(
            "{}\ntest \"leak in test\" {{\n  assert_eq(Leak(\"x\"), \"done\")\n}}",
            LEAK_ROOT
        ),
    )
    .expect("t.mlog");
    let (code, stdout, stderr) = run(&dir, &["test", "t.mlog", "--allow-unchecked"]);
    let combined = format!("{}{}", stdout, stderr);
    assert_eq!(code, 0, "the explicit opt-out runs the tests: {}", combined);
    assert!(
        combined.contains("UNCHECKED MODE") && combined.contains("--allow-unchecked"),
        "the loud banner must fire on the opt-out, got: {}",
        combined
    );
    assert!(
        dir.join("LEAKED_CLI.txt").exists(),
        "the unchecked run executes (the side effect is the point of the opt-out)"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// resume EXECUTES flow bodies — the same gate set refuses the leak.
#[test]
fn n953_cli_resume_refuses_the_leak() {
    let dir = tmp_dir("resume");
    fs::write(dir.join("r.mlog"), LEAK_ROOT).expect("r.mlog");
    let (code, stdout, stderr) = run(
        &dir,
        &["resume", "r.mlog", "--flow", "Main", "--from", "cp"],
    );
    let combined = format!("{}{}", stdout, stderr);
    assert_ne!(code, 0, "resume must refuse the leak program");
    assert!(
        combined.contains("Category A security invariant violated"),
        "the resume refusal must carry the Category-A gate, got: {}",
        combined
    );
    assert!(
        !dir.join("LEAKED_CLI.txt").exists(),
        "resume executed the leak despite the gate"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// `mlog run *.mbc`: the gates rode the COMPILE lane — a leaking program
/// never becomes bytecode; the loader refuses garbage cleanly (no panic,
/// no partial execution).
#[test]
fn n953_cli_compile_refuses_leak_and_loader_refuses_garbage() {
    let dir = tmp_dir("mbc");
    fs::write(dir.join("leak.mlog"), LEAK_ROOT).expect("leak.mlog");
    let (code, _stdout, stderr) = run(&dir, &["compile", "leak.mlog"]);
    assert_ne!(code, 0, "compile must refuse the leak program: {}", stderr);
    assert!(
        !dir.join("leak.mbc").exists(),
        "no bytecode may be produced from a refused program"
    );

    fs::write(dir.join("garbage.mbc"), b"\x00\x01not-a-program").expect("garbage");
    let (code, _stdout, stderr) = run(&dir, &["run", "garbage.mbc"]);
    assert_ne!(code, 0, "garbage bytecode must refuse");
    assert!(
        !stderr.contains("panicked"),
        "the loader refuses CLEANLY (no panic): {}",
        stderr
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The LSP surface analyzes — it never executes: no interpreter is
/// constructed anywhere in the mlog-lsp crate (the source-level pin).
#[test]
fn n953_lsp_has_no_execution_surface() {
    let lsp_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mlog-lsp/src");
    let mut hits = Vec::new();
    for entry in ["lib.rs", "main.rs"] {
        let path = lsp_src.join(entry);
        if let Ok(src) = fs::read_to_string(&path) {
            for banned in [
                "Interpreter::new(",
                "run_test_blocks(",
                "run_program",
                "resume_program",
            ] {
                if src.contains(banned) {
                    hits.push(format!("{} contains {}", entry, banned));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "the LSP must not carry an execution surface (№953 input→gate inventory): {:?}",
        hits
    );
}
