// Naryad №782 (audit-23.10 A3): the resource-limit pins. SUBPROCESS form
// by design (the fs_gate ratchet allow is scoped to this file): the
// pre-fix behavior KILLED the process (the allocator abort on the 1 TB
// repeat; the stack overflow on the unbounded CallPattern recursion) —
// an in-process test could not survive its own RED.
//
// RED-before (verified on the unpatched main d0245a4, stash-run):
//   - repeat("a", 1000000000000.0): "memory allocation of 1000000000000
//     bytes failed" (alloc abort) on BOTH backends;
//   - the R(n){return R(n+1.0)} recursion: "has overflowed its stack"
//     on BOTH backends (TW run-thread, VM main-thread).
// GREEN: the loud coded refusals `[STRING_TOO_LARGE]` /
// `[CALL_DEPTH_EXCEEDED]` with exit code 1, the process alive.
#![allow(clippy::disallowed_methods)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn mlog_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mlog"))
}

/// Write the program to a temp dir, return the .mlog path.
fn write_prog(name: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("n782_pins");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(format!("{name}.mlog"));
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(source.as_bytes()).unwrap();
    p
}

fn compile_to_mbc(mlog_path: &PathBuf) -> PathBuf {
    let out = Command::new(mlog_bin())
        .arg("compile")
        .arg(mlog_path)
        .output()
        .expect("mlog compile spawned");
    assert!(
        out.status.success(),
        "compile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    mlog_path.with_extension("mbc")
}

/// Run `mlog run <path>` with a hard timeout (the pre-fix main-loop bug
/// HUNG forever — the timeout is the RED detector for that class).
fn run_with_timeout(path: &PathBuf, secs: u64) -> (Option<i32>, String, String, bool) {
    use std::sync::mpsc;
    let mut child = Command::new(mlog_bin())
        .arg("run")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("mlog run spawned");
    // Pipe the output in a helper thread (the child may produce a lot).
    let stdout_handle = child.stdout.take().unwrap();
    let stderr_handle = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut out = String::new();
        let mut err = String::new();
        let _ = std::io::BufReader::new(stdout_handle).read_to_string(&mut out);
        let _ = std::io::BufReader::new(stderr_handle).read_to_string(&mut err);
        let _ = tx.send((out, err));
    });
    let deadline = Instant::now() + Duration::from_secs(secs);
    let outputs;
    loop {
        if Instant::now() >= deadline {
            // The RED detector for the main-loop class: the process HUNG
            // instead of the loud refusal — kill it and report the timeout.
            let _ = child.kill();
            let status = child.wait();
            let code = status.ok().and_then(|s| s.code());
            return (Some(code.unwrap_or(-1)), String::new(), String::new(), true);
        }
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok((out, err)) => {
                outputs = (out, err);
                let status = child.wait();
                let code = status.ok().and_then(|s| s.code());
                return (Some(code.unwrap_or(-1)), outputs.0, outputs.1, false);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                outputs = (String::new(), String::new());
                break;
            }
        }
    }
    let _ = child.wait();
    (None, outputs.0, outputs.1, false)
}

const REPEAT_PROG: &str = r#"
pattern Kick(s: String) -> String {
  return repeat("a", 1000000000000.0)
}
flow Main { input: String = "x" -> Kick -> output }
"#;

const PAD_LEFT_PROG: &str = r#"
pattern Kick(s: String) -> String {
  return pad_left("x", 1000000000000.0, " ")
}
flow Main { input: String = "x" -> Kick -> output }
"#;

const REC_PROG: &str = r#"
pattern R(n: Float) -> Float {
  return R(n + 1.0)
}
pattern Kick(s: String) -> Float {
  return R(0.0)
}
flow Main { input: String = "x" -> Kick -> output }
"#;

const ECHO_PROG: &str = r#"
pattern Echo(s: String) -> String {
  return s
}
flow Main { input: String = "x" -> Echo -> output }
"#;

fn assert_loud_refusal(
    code: Option<i32>,
    stderr: &str,
    timed_out: bool,
    marker: &str,
    label: &str,
) {
    assert!(
        !timed_out,
        "{label}: the run TIMED OUT — the process hung instead of the loud refusal"
    );
    let code = code.expect("exit code present");
    assert_eq!(
        code, 1,
        "{label}: expected the loud exit code 1 (process alive), got {code}; stderr: {stderr}"
    );
    assert!(
        stderr.contains(marker),
        "{label}: expected [{marker}] in stderr, got: {stderr}"
    );
}

#[test]
fn repeat_oom_guard_tw() {
    let p = write_prog("n782_repeat_tw", REPEAT_PROG);
    let (code, _out, err, to) = run_with_timeout(&p, 60);
    assert_loud_refusal(code, &err, to, "STRING_TOO_LARGE", "repeat TW");
}

#[test]
fn repeat_oom_guard_vm() {
    let p = compile_to_mbc(&write_prog("n782_repeat_vm", REPEAT_PROG));
    let (code, _out, err, to) = run_with_timeout(&p, 60);
    assert_loud_refusal(code, &err, to, "STRING_TOO_LARGE", "repeat VM");
}

#[test]
fn pad_left_oom_guard_vm() {
    let p = compile_to_mbc(&write_prog("n782_pad_vm", PAD_LEFT_PROG));
    let (code, _out, err, to) = run_with_timeout(&p, 60);
    assert_loud_refusal(code, &err, to, "STRING_TOO_LARGE", "pad_left VM");
}

#[test]
fn recursion_depth_guard_tw() {
    let p = write_prog("n782_rec_tw", REC_PROG);
    let (code, _out, err, to) = run_with_timeout(&p, 120);
    assert_loud_refusal(code, &err, to, "CALL_DEPTH_EXCEEDED", "recursion TW");
}

#[test]
fn recursion_depth_guard_vm() {
    let p = compile_to_mbc(&write_prog("n782_rec_vm", REC_PROG));
    let (code, _out, err, to) = run_with_timeout(&p, 120);
    assert_loud_refusal(code, &err, to, "CALL_DEPTH_EXCEEDED", "recursion VM");
}

/// The `while true` main-loop pin: the top-level statement grammar does
/// not carry `while` into main_code, so the eternal loop is built by
/// PATCHING a real compiled program — the self-jumping `Jump` appended to
/// main_code (an instruction that jumps onto itself: an eternal loop with
/// NO stack growth). The pre-№782 `execute_main_code` had NO iteration
/// counter — this program hung `run_bytecode` forever (the RED detector
/// = the timeout).
#[test]
fn vm_main_loop_infinite_iteration_guard() {
    use metalogos::bytecode;
    // Compile a minimal LEGAL program (the same flow shape as the other pins).
    let p = write_prog("n782_while_vm", ECHO_PROG);
    let mbc = compile_to_mbc(&p);
    let data = std::fs::read(&mbc).unwrap();
    let mut program = bytecode::Program::deserialize(&data).expect("deserialize");
    // Patch: the compiler ALWAYS appends `Halt` to main_code — replace it
    // with a self-jumping `Jump` (an instruction that jumps onto itself:
    // an eternal main loop with NO stack growth, reachable by execution).
    assert!(
        matches!(program.main_code.last(), Some(bytecode::Instruction::Halt)),
        "the compiler must end main_code with Halt"
    );
    let self_ip = program.main_code.len() - 1;
    program.main_code[self_ip] = bytecode::Instruction::Jump(self_ip);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = metalogos::run_bytecode(program);
        let _ = tx.send(out);
    });
    match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(Err(e)) => assert!(
            e.contains("possible infinite loop"),
            "expected the main-loop iteration limit message, got: {e}"
        ),
        Ok(Ok(_)) => panic!("the eternal main-loop program returned Ok — no iteration limit"),
        Err(_) => panic!(
            "the eternal main-loop program TIMED OUT — execute_main_code has no iteration counter (the №782 RED)"
        ),
    }
}
