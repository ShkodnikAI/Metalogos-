// ── №980 (issue #1994): the RUNTIME pins — the echo hand in the TW and
//    VM lanes ──────────────────────────────────────────────────────────
//
// Separate file = separate test binary = its own process env: these
// pins run with `METALOGOS_STATIC_LABEL_GATE_OFF=1` (the №952 test-only
// silencer, audit.rs check_sink_clearance) so the refusal can come ONLY
// from the runtime value-label line — the static lane is out of the
// picture by construction. Removing the echo consult in
// `interpreter/labels.rs` reddens the TW pins; removing it in
// `vm.rs::builtin_result_label` reddens the VM pins — each lane's hand
// is isolated by its own pin (the dispatch's «снять руку в одном —
// краснеет ровно этот пин»).
//
// The probes ride env("SP") (a real secret) because the RUNTIME needs a
// live value; the static-lane probes (query_param as the untrusted
// NOT-secret reason) live in tests/naryad_980_receipt_echo.rs.
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

fn assert_runtime_refusal(backend: &str, err: &str) {
    // THE PIN IS THE HAND, not «a refusal somewhere». The reason here is
    // `query_param` — untrusted (UserInput) but NOT a secret (the №977
    // probe property), so the Category-A secret line stays SILENT and the
    // refusal can come ONLY from the sink-clearance runtime line — the
    // line fed by the echo hand in this backend's lane. A secret reason
    // (env) would let the Category-A line refuse even with the hand
    // removed, and the pin would survive its own mutation.
    assert!(
        err.contains("[SINK_CLEARANCE_RUNTIME]"),
        "{backend}: expected the sink-clearance runtime refusal on the \\
         reason echo, got: {}",
        err
    );
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

/// TW: a SECRET passed AS THE REASON rides the receipt echo — the print
/// gate refuses at runtime. Mutation target: the echo consult in
/// `interpreter/labels.rs`.
#[test]
fn n980_tw_runtime_the_reason_echo_refuses() {
    seed_env();
    if let Err(e) = run_tw(REASON_ECHO) {
        assert_runtime_refusal("TW/reason-echo", &e);
    } else {
        panic!(
            "TW/reason-echo: the secret reason printed — the TW echo \
             hand is dead"
        );
    }
}

/// VM: the same probe on the executor backend. Mutation target: the
/// echo consult in `vm.rs::builtin_result_label` (the dispatch measured
/// that lane at ZERO consults of the shared tables before №980).
#[test]
fn n980_vm_runtime_the_reason_echo_refuses() {
    seed_env();
    if let Err(e) = run_vm(REASON_ECHO) {
        assert_runtime_refusal("VM/reason-echo", &e);
    } else {
        panic!(
            "VM/reason-echo: the secret reason printed — the VM echo \
             hand is dead"
        );
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
             receipt must stay legal (№325/№964) — the payload must not \
             ride the return — got: {}",
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
             receipt must stay legal (№325/№964) — the payload must not \
             ride the return — got: {}",
            e
        ),
    }
}
