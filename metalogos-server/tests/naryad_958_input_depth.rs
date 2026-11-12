// ── №958 (issue #1925, the audit 12.11 — P1): the input-depth limits ───
//
// The audit's M-1: 50 000 chained `+` or 3 000 nested `if` overflowed the
// 8 MB parser thread — `fatal runtime error: stack overflow, aborting`
// (SIGABRT, exit 134). The fix: the parser pre-scan (the ONE gate every
// source passes; `limits::max_input_depth()`, the env override
// `METALOGOS_MAX_INPUT_DEPTH`) refuses with a clean `[INPUT_TOO_DEEP]`
// naming the position; the semantic walkers carry the DepthGuard second
// line (tests/naryad_958_walk_guard.rs). The LSP rides the same parser.
//
// The mutation: scripts/mutation_verify_958.sh — M1 neuters the parser
// scan (the abort returns → these pins go red), M2 neuters the walk guard
// (the synthetic walk test aborts → red).
#![allow(clippy::disallowed_methods)]

use std::process::Command;
use std::time::Duration;

fn gen_deep_plus(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("deep_plus.mlog");
    let tail = "\nflow Main { input: String = \"x\" -> P -> output }\n";
    let body: Vec<&str> = vec!["\"a\""; 50_000];
    std::fs::write(
        &path,
        format!(
            "pattern P(x: String) -> String {{ let a = {} return a }}{}",
            body.join("+"),
            tail
        ),
    )
    .expect("write deep_plus");
    path
}

fn gen_deep_if(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("deep_if.mlog");
    let tail = "\nflow Main { input: String = \"x\" -> P -> output }\n";
    let open: Vec<&str> = vec!["if x == \"x\" then {"; 3000];
    let close: Vec<&str> = vec!["}"; 3000];
    std::fs::write(
        &path,
        format!(
            "pattern P(x: String) -> String {{ {}print(\"a\"){} return \"ok\" }}{}",
            open.join(" "),
            close.join(""),
            tail
        ),
    )
    .expect("write deep_if");
    path
}

fn run_check(file: &std::path::Path, env: Option<(&str, &str)>) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mlog"));
    cmd.args(["check", file.to_str().expect("path")]);
    cmd.env("METALOGOS_ENV_ALLOWLIST", "SP");
    if let Some((k, v)) = env {
        cmd.env(k, v);
    }
    // The abort (SIGABRT) returns 134; a hung parse would trip the
    // outer timeout — the fix must answer in well under a second.
    wait_with_timeout(cmd, Duration::from_secs(30))
}

fn wait_with_timeout(mut cmd: Command, timeout: Duration) -> (i32, String) {
    use std::io::Read;
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn mlog");
    // The readers run CONCURRENTLY with the wait: the parse-error display
    // quotes the offending SOURCE line (the deep probes sit on one
    // 200 KB line) — a pipe left unread fills and deadlocks the child.
    let mut stdout_pipe = child.stdout.take().expect("stdout");
    let mut stderr_pipe = child.stderr.take().expect("stderr");
    let out_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout_pipe.read_to_string(&mut s);
        s
    });
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr_pipe.read_to_string(&mut s);
        s
    });
    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => break status,
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("the check did not finish within {:?}", timeout);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };
    let mut out = out_reader.join().expect("out reader");
    out.push_str(&err_reader.join().expect("err reader"));
    (status.code().unwrap_or(-1), out)
}

fn tmp(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("n958_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");
    dir
}

/// THE audit probes: a clean one-line refusal, exit 1 — never the abort.
#[test]
fn n958_deep_plus_refuses_cleanly() {
    let dir = tmp("plus");
    let f = gen_deep_plus(&dir);
    let (code, out) = run_check(&f, None);
    assert_eq!(code, 1, "the refusal must be exit 1, got: {}", out);
    assert!(
        out.contains("INPUT_TOO_DEEP"),
        "the refusal must carry the stable class, got: {}",
        out
    );
    assert!(
        !out.contains("overflowed its stack") && code != 134,
        "the abort returned — the scan is gone (the mutation state?)"
    );
    assert!(
        out.contains("METALOGOS_MAX_INPUT_DEPTH"),
        "the refusal must name the env knob, got: {}",
        out
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn n958_deep_if_refuses_cleanly() {
    let dir = tmp("if");
    let f = gen_deep_if(&dir);
    let (code, out) = run_check(&f, None);
    assert_eq!(code, 1, "the refusal must be exit 1, got: {}", out);
    assert!(
        out.contains("INPUT_TOO_DEEP") && code != 134,
        "the clean refusal expected, got: {}",
        out
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The corpus is safe: a legal program with 100-level nesting parses.
#[test]
fn n958_legal_nesting_still_parses() {
    let dir = tmp("legal");
    let f = dir.join("legal.mlog");
    let open: Vec<&str> = vec!["if x == \"x\" then {"; 100];
    let close: Vec<&str> = vec!["}"; 100];
    std::fs::write(
        &f,
        format!(
            "pattern P(x: String) -> String {{ {}return upper(x){} }}\nflow Main {{ input: String = \"x\" -> P -> output }}\n",
            open.join(" "),
            close.join("")
        ),
    )
    .expect("write legal");
    let (code, out) = run_check(&f, None);
    assert_eq!(
        code, 0,
        "the legal 100-nesting program must parse, got: {}",
        out
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The env knob: METALOGOS_MAX_INPUT_DEPTH=20 refuses a 30-nesting program.
#[test]
fn n958_env_override_works() {
    let dir = tmp("env");
    let f = dir.join("mid.mlog");
    let open: Vec<&str> = vec!["if x == \"x\" then {"; 30];
    let close: Vec<&str> = vec!["}"; 30];
    std::fs::write(
        &f,
        format!(
            "pattern P(x: String) -> String {{ {}return upper(x){} }}\nflow Main {{ input: String = \"x\" -> P -> output }}\n",
            open.join(" "),
            close.join("")
        ),
    )
    .expect("write mid");
    let (code, out) = run_check(&f, Some(("METALOGOS_MAX_INPUT_DEPTH", "20")));
    assert_eq!(
        code, 1,
        "the env override must lower the limit, got: {}",
        out
    );
    assert!(out.contains("INPUT_TOO_DEEP"), "got: {}", out);
    let _ = std::fs::remove_dir_all(&dir);
}
