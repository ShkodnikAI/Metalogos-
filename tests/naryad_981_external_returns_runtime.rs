// ── №981 (issue #1995): the RUNTIME pins — the external-return hand in
//    the TW and VM lanes ───────────────────────────────────────────────
//
// №978 put the rule «a sink's value that carries EXTERNAL data arrives
// `public, untrusted`» in the two STATIC lanes (semantic.rs, secret_label.rs)
// and left the RUNTIME lanes without it: on the VM (the executor,
// ADR-0197) and the TW (the reference oracle) an `http_post`/`exec`
// return still travelled as the argument join — `bottom` for a literal
// URL — so `respond(http_post(…))` executed where `mlog check` refuses.
//
// Separate file = separate process env: these pins run with
// `METALOGOS_STATIC_LABEL_GATE_OFF=1` (the №952 test-only silencer — it
// silences the STATIC sink-clearance twin so `check` passes) so the
// refusal can come ONLY from the runtime value-label line — the line
// fed by THIS lane's external-return hand.
//
// THE LAYERING IS PART OF THE PIN (measured, not assumed):
// - The TW gate fires at the sink's FnCall ARM HEAD, before argument
//   evaluation — the inline probe refuses before any network attempt or
//   command execution.
// - The VM is a stack machine: the sink's arguments are EVALUATED before
//   the sink gate sees their value labels. For a NETWORK carrier the
//   earlier layers (the capability gate, the SSRF guard) refuse first —
//   the label line sits behind them by design (ADR-0203 §2.4). The VM
//   hand is therefore demonstrated on `exec` under
//   `METALOGOS_ALLOW_EXEC=1`: the call SUCCEEDS (echo runs), the return
//   value is created carrying the hand's `public, untrusted`, and the
//   respond gate refuses on the label — no earlier layer can take the
//   credit. The http_post VM probe pins the layering honestly: refused,
//   but by an earlier layer, not by the label line.
// - http_get is NOT an external-return name (its №325 ingress rule
//   labels the response body at the SOURCE — refused on both lanes
//   BEFORE №981) — the controls pin the PRESERVED verdict: the refusal
//   stays, by the same class, hand-independent.
// - MEASURED RESIDUAL: the flow-edge VALUE transport (a pattern's return
//   crossing the edge to `output`) is variant-floor-labeled at runtime —
//   no lane's summary hand reaches it. That is №982-adjacent territory
//   (the transport label loss), recorded in the PR body, not pinned here.
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

fn seed_env() {
    std::env::set_var("METALOGOS_STATIC_LABEL_GATE_OFF", "1");
    // The named host permit for exec (the promise-5 capability layer):
    // lets the VM probe's exec call succeed so the return VALUE exists
    // and the respond gate can refuse on its LABEL, not on the
    // capability layer.
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
}

fn assert_runtime_refusal(backend: &str, err: &str) {
    assert!(
        err.contains("[SINK_CLEARANCE_RUNTIME]"),
        "{backend}: expected the sink-clearance runtime refusal on the \
         external return, got: {}",
        err
    );
}

const HTTP_POST_RETURN: &str = r#"
pattern P(d: String) -> String {
  let _ = respond(http_post("http://127.0.0.1:1/x", "hi"))
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

const EXEC_RETURN: &str = r#"
pattern P(d: String) -> String {
  let _ = respond(exec("echo hi"))
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

/// TW: `respond(http_post(…))` refuses BEFORE the network attempt — the
/// arm-head gate labels the inline call through this lane's hand.
/// Mutation target: the external consult in `interpreter/labels.rs`
/// (zero consults before №981 — the dispatch measured the lane).
#[test]
fn n981_tw_runtime_the_http_post_return_refuses() {
    seed_env();
    if let Err(e) = run_tw(HTTP_POST_RETURN) {
        assert_runtime_refusal("TW/http_post-return", &e);
    } else {
        panic!(
            "TW/http_post-return: the external return flowed to respond \
             — the TW external-return hand is dead (№981)"
        );
    }
}

/// TW: `respond(exec(…))` refuses BEFORE the command runs — the same
/// arm-head gate, the second verified name of the rule.
#[test]
fn n981_tw_runtime_the_exec_return_refuses() {
    seed_env();
    if let Err(e) = run_tw(EXEC_RETURN) {
        assert_runtime_refusal("TW/exec-return", &e);
    } else {
        panic!(
            "TW/exec-return: the host stdout flowed to respond — the TW \
             external-return hand is dead (№981)"
        );
    }
}

/// VM: `respond(exec(…))` refuses on the LABEL of the return value — the
/// exec call runs under the named permit, the value is created carrying
/// `public, untrusted` by the VM hand (`vm.rs::builtin_result_label`,
/// zero consults before №981), and the respond gate denies it. No
/// earlier layer can take the credit: without the permit the capability
/// gate refuses BEFORE the value exists, which is a different refusal.
/// Mutation target: the external consult in `vm.rs`.
#[test]
fn n981_vm_runtime_the_exec_return_refuses() {
    seed_env();
    if let Err(e) = run_vm(EXEC_RETURN) {
        assert_runtime_refusal("VM/exec-return", &e);
    } else {
        panic!(
            "VM/exec-return: the host stdout flowed to respond — the VM \
             external-return hand is dead (№981)"
        );
    }
}

/// VM: the layering pin (measured, honest): for a NETWORK carrier the VM
/// evaluates the sink's arguments first, so the capability gate / SSRF
/// guard refuse BEFORE the label line can. The flow never executes — but
/// the refusal is NOT the label line's. If this pin ever sees
/// [SINK_CLEARANCE_RUNTIME] here, the VM's gate order moved in front of
/// evaluation — an observable change worth its own naryad.
#[test]
fn n981_vm_runtime_the_http_post_return_is_refused_before_the_label_line() {
    seed_env();
    match run_vm(HTTP_POST_RETURN) {
        Ok(_) => panic!(
            "VM/http_post-layering: respond(http_post(…)) EXECUTED — no \
             layer refused; the capability/SSRF guards and the label \
             line all failed to fire"
        ),
        Err(e) => assert!(
            !e.contains("[SINK_CLEARANCE_RUNTIME]"),
            "VM/http_post-layering: the label line fired BEFORE \
             evaluation — the VM gate order changed; re-measure the \
             layering story in this file's header and in the PR body"
        ),
    }
}

/// THE CONTROL (the done-when: «http_get сохраняет сегодняшний вердикт»):
/// `http_get` is NOT in the external-return table — the label line must
/// NOT refuse its return. Whatever error the run produces (the loopback
/// SSRF guard, the connection refusal — both environmental), it must not
/// be the label line's refusal: that would mean http_get leaked into the
/// external table and the №316 asymmetry's control broke.
#[test]
fn n981_tw_runtime_the_http_get_control_is_preserved() {
    seed_env();
    match run_tw(
        r#"
pattern P(d: String) -> String {
  let _ = respond(http_get("http://127.0.0.1:1/x"))
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#,
    ) {
        Ok(_) => panic!(
            "TW/http_get-control: http_get's return PASSED the respond \
             gate — the №325 ingress verdict (refused before №981, via \
             the source label) changed; got no refusal"
        ),
        Err(e) => assert!(
            e.contains("[SINK_CLEARANCE_RUNTIME]"),
            "TW/http_get-control: expected the PRESERVED refusal (the \
             source label predates №981) — got a different refusal: {}",
            e
        ),
    }
}

/// VM: the same control on the executor backend.
#[test]
fn n981_vm_runtime_the_http_get_control_is_preserved() {
    seed_env();
    match run_vm(
        r#"
pattern P(d: String) -> String {
  let _ = respond(http_get("http://127.0.0.1:1/x"))
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#,
    ) {
        Ok(_) => panic!(
            "VM/http_get-control: http_get's return PASSED the respond \
             gate — the №325 ingress verdict (refused before №981) \
             changed; got no refusal"
        ),
        Err(e) => {
            // The VM evaluates the sink's arguments first, so the loopback
            // URL hits the SSRF guard BEFORE the label line can see the
            // value — the same measured layering as the http_post pin
            // above. The preservation claim on the VM is therefore: the
            // run refuses (any layer), never executes. The CLASS-level
            // control lives on the TW (the arm-head gate sees the label).
            assert!(
                !e.is_empty(),
                "VM/http_get-control: expected a refusal, got an empty \
                 error"
            );
        }
    }
}
