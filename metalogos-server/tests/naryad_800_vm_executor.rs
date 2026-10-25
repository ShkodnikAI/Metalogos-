// ── Naryad №800 (wave 65, P1 — audit-23.10 C5-b V3, ADR-0197): run / eval / REPL execute on the VM; --reference (TW) is the transitional oracle lane ──
//
// Contract (issue #1547, the C5 (b) owner decision 23.10, ADR-0197 §2/§5):
// 1. `mlog run` (source), `mlog eval`, `mlog repl` execute on the VM — the
//    SAME executor as `serve` and `mcp-serve` (№795). TW stops being a user
//    command executor and becomes the REFERENCE ORACLE: it stays in CI
//    (crosscheck, diff-fuzzer, doc-tests) and behind `--reference`.
// 2. `--reference` executes on TW with the loud banner "reference backend
//    (TW) — for comparison only". The flag is TRANSITIONAL (two release
//    windows from this landing, ADR-0197 §4.3).
// 3. REPL: incremental VM state between inputs — the persistent Vm carries
//    globals/patterns/learnables/memory across lines (the session chunk
//    compiler; a chunk compiles against the session's symbol tables and
//    run_chunk appends the chunk's tables to the live VM).
// 4. The backend detector: METALOGOS_BACKEND_ECHO=1 makes run/eval/repl
//    print the executor to stderr ("[backend: vm]" / "[backend:
//    reference-tw]") — the test hook the naryad names; silent by default.
// 5. Programs where TW and VM diverged now behave in `run` as they do in
//    `serve` (the CHANGELOG Changed row); the crosscheck corpus (examples/)
//    keeps its expected outputs — run = VM must reproduce them.
//
// RED-before (the №772 form): these pins were run against the pre-№800
// binary — the echo hook did not exist (no "[backend: vm]" line) and the
// --reference flag was a clap error; every pin below was RED before the
// flip landed.

#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn mlog_bin() -> &'static str {
    env!("CARGO_BIN_EXE_mlog")
}

/// A temp dir for the fixture programs (the №475 CLI loader boundary:
/// argv paths, author-controlled — the real FS is the honest fixture).
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("n800-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        TempDir(dir)
    }
    fn write(&self, name: &str, content: &str) -> PathBuf {
        let p = self.0.join(name);
        fs::write(&p, content).expect("write fixture");
        p
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Run the real binary with env vars applied.
fn run_mlog(args: &[&str], envs: &[(&str, &str)]) -> (String, String, Option<i32>) {
    let out = Command::new(mlog_bin())
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .expect("spawn mlog");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code(),
    )
}

// ── 1. `mlog run` defaults to the VM ─────────────────────────────────

#[test]
fn n800_run_defaults_to_vm() {
    let td = TempDir::new("run-vm");
    let p = td.write(
        "hello.mlog",
        r#"entity greeting: String = "Hello, Metalogos!"

pattern Shout(s: String) -> String { return upper(s) + "!" }

flow Main { input: String = greeting -> Shout -> output }
"#,
    );

    let (stdout, stderr, code) = run_mlog(
        &["run", p.to_str().unwrap()],
        &[("METALOGOS_BACKEND_ECHO", "1")],
    );
    assert_eq!(code, Some(0), "run failed: {stderr}");
    // The backend detector: the default lane IS the VM.
    assert!(
        stderr.contains("[backend: vm]"),
        "expected the [backend: vm] detector line on stderr, got: {stderr}"
    );
    assert!(
        !stderr.contains("[backend: reference-tw]"),
        "the reference lane must not fire without --reference: {stderr}"
    );
    // The m1_hello corpus expectation — the VM reproduces the TW answer.
    assert_eq!(stdout.trim(), "HELLO, METALOGOS!!", "stdout: {stdout}");
}

// ── 2. `mlog run --reference` — the TW oracle lane with the banner ────

#[test]
fn n800_run_reference_flag_is_tw_with_banner() {
    let td = TempDir::new("run-ref");
    let p = td.write(
        "hello.mlog",
        r#"entity greeting: String = "Hello, Metalogos!"

pattern Shout(s: String) -> String { return upper(s) + "!" }

flow Main { input: String = greeting -> Shout -> output }
"#,
    );

    let (stdout, stderr, code) = run_mlog(
        &["run", "--reference", p.to_str().unwrap()],
        &[("METALOGOS_BACKEND_ECHO", "1")],
    );
    assert_eq!(code, Some(0), "run --reference failed: {stderr}");
    assert!(
        stderr.contains("[backend: reference-tw]"),
        "expected the [backend: reference-tw] detector line, got: {stderr}"
    );
    assert!(
        stderr.contains("reference backend (TW) — for comparison only"),
        "expected the ADR-0197 banner on stderr, got: {stderr}"
    );
    assert_eq!(stdout.trim(), "HELLO, METALOGOS!!");
}

// ── 3. `mlog eval` defaults to the VM ────────────────────────────────

#[test]
fn n800_eval_defaults_to_vm() {
    let td = TempDir::new("eval-vm");
    // The few-shot cache makes this eval deterministic with NO external
    // LLM: the adapt during declaration execution feeds the example, the
    // eval dataset input hits the cache exactly (the VM call_llm checks
    // few-shot first — the same contract as the TW invoke_learnable).
    let p = td.write(
        "cls.mlog",
        r#"learnable pattern cls(text: String) -> String { prompt: "classify the text" }

adapt cls add_example("good day", "positive")

eval cls { dataset: [("good day", "positive")], metric: accuracy, threshold: 0.5 }
"#,
    );

    let (stdout, stderr, code) = run_mlog(
        &["eval", p.to_str().unwrap()],
        &[("METALOGOS_BACKEND_ECHO", "1")],
    );
    assert_eq!(code, Some(0), "eval failed: {stderr}");
    assert!(
        stderr.contains("[backend: vm]"),
        "expected the [backend: vm] detector line on stderr, got: {stderr}"
    );
    assert!(
        stdout.contains("PASS"),
        "eval report must pass (the few-shot cache answers 'positive'), got: {stdout}"
    );
}

// ── 4. `mlog eval --reference` — the TW oracle lane ──────────────────

#[test]
fn n800_eval_reference_flag_is_tw_with_banner() {
    let td = TempDir::new("eval-ref");
    let p = td.write(
        "cls.mlog",
        r#"learnable pattern cls(text: String) -> String { prompt: "classify the text" }

adapt cls add_example("good day", "positive")

eval cls { dataset: [("good day", "positive")], metric: accuracy, threshold: 0.5 }
"#,
    );

    let (stdout, stderr, code) = run_mlog(
        &["eval", "--reference", p.to_str().unwrap()],
        &[("METALOGOS_BACKEND_ECHO", "1")],
    );
    assert_eq!(code, Some(0), "eval --reference failed: {stderr}");
    assert!(stderr.contains("[backend: reference-tw]"), "got: {stderr}");
    assert!(
        stderr.contains("reference backend (TW) — for comparison only"),
        "got: {stderr}"
    );
    assert!(stdout.contains("PASS"), "stdout: {stdout}");
}

// ── 5. REPL on the VM: incremental state between inputs ──────────────

#[test]
fn n800_repl_defaults_to_vm_with_incremental_state() {
    // The phase72 shape on the VM lane: memorize in line 1, the query
    // entity in line 2, the recall flow in line 3 — the persistent Vm
    // carries memory + globals across the chunks.
    let input = "memorize \"the cat sat on the mat\" with priority=1.0\n\
                 entity query: String = \"the cat sat\"\n\
                 flow Main { input: String = query -> recall -> output }\n\
                 exit\n";
    let (stdout, stderr, _code) =
        run_mlog_piped_repl(&[], input, &[("METALOGOS_BACKEND_ECHO", "1")]);
    assert!(
        stderr.contains("[backend: vm]"),
        "expected the [backend: vm] detector line on stderr, got: {stderr}"
    );
    assert!(
        stdout.contains("cat sat"),
        "the VM REPL must carry state between inputs (the recall flow answers), got: {stdout}"
    );
}

// ── 6. REPL --reference — the TW oracle lane ─────────────────────────

#[test]
fn n800_repl_reference_flag_is_tw() {
    let input = "entity greeting: String = \"hi\"\nexit\n";
    let (stdout, stderr, _code) =
        run_mlog_piped_repl(&["--reference"], input, &[("METALOGOS_BACKEND_ECHO", "1")]);
    assert!(
        stderr.contains("[backend: reference-tw]"),
        "expected the reference-tw detector for `mlog repl --reference`, got: {stderr}"
    );
    let _ = (stdout, _code);
}

/// Feed stdin lines to the piped REPL (extra CLI args go before the input).
fn run_mlog_piped_repl(
    extra_args: &[&str],
    input: &str,
    envs: &[(&str, &str)],
) -> (String, String, Option<i32>) {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(mlog_bin())
        .arg("repl")
        .args(extra_args)
        .envs(envs.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mlog repl");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input.as_bytes()).expect("write repl input");
    }
    let out = child.wait_with_output().expect("repl output");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code(),
    )
}

// ── 7. The session compiler: cross-line symbols resolve against the
//       live tables (the lib-level pins for the chunk machinery) ────────

#[test]
fn n800_vm_repl_cross_line_pattern_call() {
    // Line 1 declares a pattern; line 2's entity initializer CALLS it —
    // the chunk compiler seeds the session base (the name resolves to the
    // live table index) and the persistent VM executes the call against
    // the appended table. A stale-compile or misbound index would answer
    // [UNDEFINED_FUNCTION] or Unit.
    let mut repl = metalogos::VmRepl::new();
    repl.feed_line("pattern double(x: Float) -> Float { return x + x }")
        .expect("line 1 must compile");
    repl.feed_line("entity v: Float = double(21.0)")
        .expect("line 2 must resolve the line-1 pattern");
    let out = repl
        .feed_line("flow Main { input: Float = v -> output }")
        .expect("line 3 must run the flow");
    let out = out.expect("the flow must produce output");
    assert!(
        out.contains("42"),
        "the cross-line pattern call must answer 42, got: {out}"
    );
}

#[test]
fn n800_vm_repl_redefinition_shadows() {
    // The TW redefinition semantics: a later binding of the same name
    // wins from that line on (the session compiler shadows the base
    // entry with a fresh slot).
    let mut repl = metalogos::VmRepl::new();
    repl.feed_line("entity x: String = \"first\"").unwrap();
    repl.feed_line("entity x: String = \"second\"").unwrap();
    let out = repl
        .feed_line("flow Main { input: String = x -> output }")
        .unwrap()
        .expect("flow output");
    assert!(
        out.contains("second"),
        "the redefinition must win, got: {out}"
    );
}

#[test]
fn n800_vm_repl_undefined_name_refuses_loud() {
    // The REPL chunk path hits the loud LoadGlobalByName backstop — the
    // same [UNDEFINED_VARIABLE] the TW REPL answers at runtime.
    let mut repl = metalogos::VmRepl::new();
    let err = repl
        .feed_line("entity u: String = no_such_name_anywhere")
        .expect_err("an undefined name must refuse");
    assert!(
        err.contains("UNDEFINED_VARIABLE"),
        "the VM REPL refuses with the stable code, got: {err}"
    );
}

#[test]
fn n800_vm_repl_memory_persists_across_lines() {
    // The memorize → recall carry-over (the phase72 contract) on the VM
    // session: the persistent Vm's memory survives across the chunks.
    let mut repl = metalogos::VmRepl::new();
    repl.feed_line("memorize \"the cat sat on the mat\" with priority=1.0")
        .unwrap();
    repl.feed_line("entity query: String = \"the cat sat\"")
        .unwrap();
    let out = repl
        .feed_line("flow Main { input: String = query -> recall -> output }")
        .unwrap()
        .expect("recall output");
    assert!(
        out.contains("cat sat"),
        "the VM memory must persist across chunks, got: {out}"
    );
}
