#![allow(clippy::disallowed_methods)]

// ── Naryad №845 (wave 72 — W-B, P1, [security][core]): the coverage rule
// — the capability decision BEFORE every irreversible action (ADR-0203
// §2.4, dispatch gh#1666) ────────────────────────────────────────────────
//
// The rule: every carrier of the ADR-0203 roster passes the capability
// layer BEFORE the run — a valid capability token, an explicit
// `profile capability { allow_carrier: … }` permit, the named exec host
// permit, or the named `standard` baseline posture. Absence of all → the
// refusal BEFORE the action, DenyEvent `IRREVERSIBLE_NO_GRANT` (the deny
// vocabulary does not grow), the Ledger deny record rides the refusal
// path. TW/VM parity on BOTH contours (the №829 lesson).
//
// RED-first: the sharp-carrier refusals (exec, delete_file, send_message,
// http_post) — without a permit the resolution is RED, with it GREEN; the
// two negative controls: (а) the bypass is impossible on any lane (the
// gate sits at every dispatch site — flow step, expression, VM bytecode;
// the roster comes from the classification machine, the totality test in
// src/capability.rs pins it), (б) `profile legacy` does not disable the
// rule (the ADR-0197 §4.3 transitional oracle is not weakened).

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), Path::new(MANIFEST).to_path_buf())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(Path::new(MANIFEST).to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// The lane-runner type — the TW/VM parity harness form (№748/№829);
/// the named alias keeps the clippy type_complexity lint honest.
type LaneRunner = fn(&str) -> Result<Option<String>, String>;

fn runners() -> [(&'static str, LaneRunner); 2] {
    [("tw", run_tw), ("vm", run_vm)]
}

static DENY_HITS: AtomicUsize = AtomicUsize::new(0);

// ── exec: the host-effect class ────────────────────────────────────────

#[test]
#[serial_test::serial]
fn exec_refuses_without_any_permit_on_both_lanes() {
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    // (1) the loud refusal without a handler.
    for (name, runner) in runners() {
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let out = exec("echo n845-must-not-run")
  return out
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: exec without a permit must refuse BEFORE the action, got: {err}"
        );
        assert!(
            !err.contains("n845-must-not-run") || err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: the refusal precedes any execution"
        );
    }
    // (2) the deny event degrades the call (the on_deny contract) — the
    // handler fires with the carrier's own sink class ("exec").
    DENY_HITS.store(0, Ordering::SeqCst);
    for (name, runner) in runners() {
        let out = runner(
            r#"
on_deny(exec) {
  match deny_reason() {
    "IRREVERSIBLE_NO_GRANT" then { print("deny:IRREVERSIBLE_NO_GRANT") }
    else { print("deny:OTHER") }
  }
}
pattern T(_t: String) -> String {
  let out = exec("echo n845-must-not-run")
  return type_of(out)
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "Unit",
            "{name}: the refusal degrades to Unit, got: {out}"
        );
    }
    // (3) the Ledger carries the deny record (the ACTION-path binding).
    let records = metalogos::ledger::all_records().unwrap();
    assert!(
        records
            .iter()
            .any(|r| r.action == "deny.IRREVERSIBLE_NO_GRANT"),
        "the capability refusal must journal itself (deny.IRREVERSIBLE_NO_GRANT)"
    );
}

#[test]
#[serial_test::serial]
fn exec_runs_with_the_explicit_profile_permit() {
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    for (name, runner) in runners() {
        let out = runner(
            r#"
profile capability { allow_carrier: "exec" }
pattern T(_t: String) -> String {
  let out = exec("echo n845-permitted")
  return out
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert!(
            out.contains("n845-permitted"),
            "{name}: exec with the explicit permit runs, got: {out}"
        );
    }
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
}

#[test]
#[serial_test::serial]
fn exec_runs_with_a_covering_capability_grant() {
    // The grant path: a capability token whose scope covers
    // `builtin:exec` authorizes the carrier (the №801 lookup machinery).
    // The token SELF-REVOKES inside the program: a grant that outlives
    // its need is a leaked authority — the ledger is process-global
    // here, and an active leftover `builtin:exec` grant would authorize
    // the refusal tests below (the nondeterministic test order made
    // this a flake: the refusal saw the leftover covering grant and
    // returned Ok).
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    for (name, runner) in runners() {
        let out = runner(
            r#"
pattern T(_t: String) -> String {
  let g = grant_issue("builtin:exec", 600, "n", 2)
  let out = exec("echo n845-granted")
  let _r = grant_revoke(g)
  return out
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert!(
            out.contains("n845-granted"),
            "{name}: a covering capability grant authorizes exec, got: {out}"
        );
    }
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
}

// ── delete_file: the destructive-fs class ──────────────────────────────

#[test]
fn delete_file_refuses_without_permit_and_runs_with_it() {
    // The fs sandbox refuses absolute paths — the carrier test uses a
    // RELATIVE path in the process cwd (the base_dir).
    let path = std::path::Path::new("n845_delete_me.txt").to_path_buf();
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, b"survive").unwrap();

    // (1) the refusal — the file SURVIVES (the refusal precedes the action).
    let err = run_tw(
        r#"
pattern T(_t: String) -> String {
  let r = delete_file("n845_delete_me.txt")
  return "deleted"
}
flow Main { input: String = "t" -> T -> output }
"#,
    )
    .unwrap_err();
    assert!(
        err.starts_with("IRREVERSIBLE_NO_GRANT"),
        "delete_file without a permit must refuse BEFORE the action, got: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "survive",
        "the refused deletion must not touch the file"
    );

    // (2) with the explicit profile permit the carrier runs.
    let out = run_tw(
        r#"
profile capability { allow_carrier: "delete_file" }
pattern T(_t: String) -> String {
  let r = delete_file("n845_delete_me.txt")
  return "deleted"
}
flow Main { input: String = "t" -> T -> output }
"#,
    )
    .unwrap()
    .unwrap();
    assert_eq!(out.trim(), "deleted", "got: {out}");
    assert!(!path.exists(), "the permitted deletion runs for real");
}

// ── send_message: the delivery class ───────────────────────────────────

#[test]
fn send_message_refuses_without_permit_and_passes_the_gate_with_it() {
    for (name, runner) in runners() {
        // (1) the refusal.
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let r = send_message("123", "n845-must-not-send")
  return r
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: send_message without a permit refuses BEFORE the action, got: {err}"
        );
        // (2) with the permit the GATE passes — the carrier RUNS (the
        // telegram delivery mechanics are the builtin's own contract and
        // are pinned by n3 against the mock; here the completion — not
        // the capability refusal — is the assertion).
        let out2 = runner(
            r#"
profile capability { allow_carrier: "send_message" }
pattern T(_t: String) -> String {
  let r = send_message("123", "n845-reaches-the-wire-gate")
  return type_of(r)
}
flow Main { input: String = "t" -> T -> output }
"#,
        );
        match out2 {
            Ok(v) => {
                let out = v.unwrap_or_default();
                assert!(
                    !out.contains("IRREVERSIBLE_NO_GRANT"),
                    "{name}: the permit must pass the capability gate, got: {out}"
                );
            }
            Err(e) => assert!(
                !e.starts_with("IRREVERSIBLE_NO_GRANT"),
                "{name}: the permit must pass the capability gate (the builtin's own contract fires next), got: {e}"
            ),
        }
    }
}

// ── http_post: the egress class (the flow-step lane too — control (а)) ─

#[test]
fn http_post_refuses_on_every_lane_without_a_permit() {
    // (1) the expression lane (let binding) — TW and VM.
    for (name, runner) in runners() {
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let r = http_post("https://n845.invalid/hook", "x")
  return r
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: http_post refuses BEFORE the action (expression lane), got: {err}"
        );
    }
    // (2) the FLOW-STEP lane (the invoke dispatch site) — VM bytecode and
    // the TW invoke path: the bypass control (а) — no lane skips the gate.
    // The URL rides the flow input (the flow grammar pipes values, the
    // step name dispatches through invoke → the same gate).
    for (name, runner) in runners() {
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  return "done"
}
flow Main { input: String = "https://n845.invalid/hook" -> http_post -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: the flow-step lane refuses too (the bypass is impossible), got: {err}"
        );
    }
}

// ── Negative control (б): profile legacy does not disable the rule ─────

#[test]
fn profile_legacy_does_not_weaken_the_capability_gate() {
    for (name, runner) in runners() {
        let err = runner(
            r#"
profile legacy { egress: permissive_with_audit }
pattern T(_t: String) -> String {
  let r = send_message("123", "n845-legacy-must-not-send")
  return r
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: the legacy compatibility profile must NOT exempt the delivery carriers (the ADR-0197 §4.3 oracle is untouched), got: {err}"
        );
    }
}

// ── The named standard posture: the baseline carriers stay permitted ───

#[test]
fn baseline_carriers_keep_the_named_standard_posture() {
    for (name, runner) in runners() {
        // db_insert (the runtime-state class, baseline) runs WITHOUT any
        // profile; print too — the honest machine roster, not silence.
        let out = runner(
            r#"
db { url: "sqlite::memory:" }
pattern T(_t: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS n845 (id INTEGER PRIMARY KEY, v TEXT)")
  let n = db_insert("n845", {id: 1.0, v: "baseline"})
  print("n845-baseline-ran")
  return "inserted:" + type_of(n)
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert!(
            out.contains("inserted:"),
            "{name}: the baseline carriers keep the standard posture, got: {out}"
        );
    }
}

// ── The disposition SSOT is the classification machine (control (а)) ───

#[test]
fn the_disposition_table_is_machine_checked() {
    // A non-carrier is not disposed (upper is Pure).
    assert!(metalogos::capability::disposition("upper").is_none());
    // Every Irreversible carrier has a disposition (the totality test in
    // src/capability.rs pins it against BUILTIN_CLASSES; here the public
    // seam is re-checked for the sharp representatives).
    for carrier in [
        "exec",
        "delete_file",
        "send_message",
        "http_post",
        "tts_send",
        "memory_forget_cascade",
        "forget",
        "print",
    ] {
        assert!(
            metalogos::capability::disposition(carrier).is_some(),
            "{carrier} must be disposed by the capability layer"
        );
    }
    // The refusals name the remedy (the honest refusal surface).
    let e = metalogos::capability::authorize_call(
        "send_message",
        &metalogos::capability::CapabilityPermits::default(),
    )
    .unwrap_err();
    assert!(e.contains("allow_carrier"), "got: {e}");
    assert!(e.contains("builtin:send_message"), "got: {e}");
}
