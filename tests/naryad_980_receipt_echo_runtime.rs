// ── №980 (issue #1994): the RUNTIME pins — FLIPPED BY №983
//    VM lanes ──────────────────────────────────────────────────────────
//
// Separate file = separate test binary = its own process env: these
// pins run with `METALOGOS_STATIC_LABEL_GATE_OFF=1` (the №952 test-only
// silencer) so any refusal would come ONLY from the runtime value-label
// line.
//
// №983 (issue #1997) removed the ECHO the hand guarded: the return is
// the machine id (`qrt-…`), the reason rides the audit journal on
// stderr. The two echo-refusal pins flipped to their negations: the
// reason-echo probe now must PASS (no echo exists — nothing to
// launder); a refusal here means the reason re-entered the return
// WITHOUT the hand (the №977 hole re-opened). The mutation that
// re-opens the hole is caught by the №983 pins' machine-id format
// assertions (tests/naryad_983_quarantine_machine_id.rs), which also
// carry the stderr journal test. The payload controls are unchanged.
#![allow(clippy::disallowed_methods)]

use std::path::Path;

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program(source)
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(Path::new("examples").to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// The probe secret (the audit PoC convention: SP=TOPSECRET123) + the
/// static-gate silencer, per process.
fn seed_env() {
    std::env::set_var("SP", "TOPSECRET123");
    std::env::set_var("METALOGOS_STATIC_LABEL_GATE_OFF", "1");
}

// THE PROBE SHAPE: the quarantine call INLINED into the sink argument —
// `print(quarantine_write(...))`. The runtime gate at the sink's FnCall
// arm labels its arguments through `label_of_expr` (the walk), and for a
// call expression that walk consults THIS lane's echo hand directly. A
// `let r = …; print(r)` shape would depend on the variable's walk-env
// label, which a builtin result does not populate on TW — the pin would
// survive its own mutation.
const REASON_ECHO: &str = r#"
pattern P(d: String) -> String {
  let _ = print(quarantine_write("q", query_param("r")))
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

const PAYLOAD_RECEIPT: &str = r#"
pattern P(d: String) -> String {
  let _ = print(quarantine_write(env("SP"), "gdpr revoked by patient"))
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

/// TW — FLIPPED: since №983 there is NO echo — the untrusted reason
/// never rides the return, the receipt (the machine id) stays bottom
/// and the print passes. A refusal here means the reason re-entered the
/// return WITHOUT the hand (the №977 hole re-opened, worse than before:
/// nothing labels it anymore).
#[test]
fn n980_tw_runtime_the_reason_echo_refuses() {
    seed_env();
    match run_tw(REASON_ECHO) {
        Ok(_) => {}
        Err(e) => panic!(
            "TW/reason-echo: the receipt refusal fired WITHOUT the hand — \
             the reason must not ride the return (the №977 hole \
             re-opened by a body-only change) — got: {}",
            e
        ),
    }
}

/// VM — FLIPPED: the same negation on the executor backend.
#[test]
fn n980_vm_runtime_the_reason_echo_refuses() {
    seed_env();
    match run_vm(REASON_ECHO) {
        Ok(_) => {}
        Err(e) => panic!(
            "VM/reason-echo: the receipt refusal fired WITHOUT the hand — \
             the №977 hole re-opened on the executor — got: {}",
            e
        ),
    }
}

/// TW: the №964 motivating case holds at runtime too — the secret is
/// the PAYLOAD (args[0]), the reason is a benign literal, the receipt
/// stays bottom and the print stays clean. A `from=0` regression (the
/// payload riding the echo) reddens this pin.
#[test]
fn n980_tw_runtime_the_payload_stays_a_receipt() {
    seed_env();
    match run_tw(PAYLOAD_RECEIPT) {
        Ok(_) => {}
        Err(e) => panic!(
            "TW/payload-receipt: quarantining a secret and logging the \
             receipt must stay legal (№325/№964) — since №983 neither the \
             payload NOR the reason rides the return — got: {}",
            e
        ),
    }
}

/// VM: the same payload control on the executor backend.
#[test]
fn n980_vm_runtime_the_payload_stays_a_receipt() {
    seed_env();
    match run_vm(PAYLOAD_RECEIPT) {
        Ok(_) => {}
        Err(e) => panic!(
            "VM/payload-receipt: quarantining a secret and logging the \
             receipt must stay legal (№325/№964) — since №983 neither the \
             payload NOR the reason rides the return — got: {}",
            e
        ),
    }
}
