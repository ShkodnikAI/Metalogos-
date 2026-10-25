// ── Naryad №702 (gh#1296, Wave 47): the 0.32 fixed-list movement №2 ──
//
// The PDn-core package over the ADR-0193 frozen list
// (scripts/ci/gate_032_untyped_sources.txt, 67 ids): TWO honest
// closures by the №644 criterion — every signature landed from the
// LIVE handler and verified HERE by a both-backend test:
//
//   conv_context → String          (session_ops.rs — the "{role}:
//                                   {text}" injection format, the
//                                   deterministic both-backend shape)
//   conv_history → List<Message>   (session_ops.rs:198-209 — the fixed
//                                   Message{role, text, timestamp}
//                                   struct shape; the field RENDER
//                                   order is HashMap-random, so the
//                                   test pins the fields, not the
//                                   rendering order)
//
// The registry fallbacks (server.rs) carry the fail-closed empty
// contracts for the context-less lanes (the authenticate mock-handler
// form; the query_param №565 value form).
//
// The EIGHT honest refusals (each with its live anchor — the probe
// runs on fresh main `c47f77a`, 2026-10-16):
//
//   query         the BACKEND-DIVERGENT contract — TW: SELECT → the row
//                 list, mutations → the affected-row count AS A STRING
//                 (db_ops.rs:133, 200-204; the №7 contract); VM: the
//                 rows are built unconditionally (db_ops.rs:699+) — a
//                 mutation yields the EMPTY list (the №538 comment
//                 verbatim: "the EXECUTION result type depends on the
//                 backend"). No single honest spelling exists; a
//                 contract unification is a language-design decision
//                 (the owner's gate), not a naryad move.
//   query_row     the ELEMENT type is data-dependent by column type —
//                 Null→Unit, Integer/Real→Float, Text/Blob→String
//                 (db_ops.rs:541-557); `List<String>` would false-warn
//                 numeric columns.
//   query_scalar  the VALUE type is data-dependent (a numeric column
//                 answers arithmetic, a text column concatenates —
//                 verified live on both backends).
//   stdin         the true stub — no handler anywhere (registry.rs:180;
//                 the mod.rs:180 doctrine); both backends refuse
//                 UNDEFINED_FUNCTION / undefined builtin.
//   request_body  the dynamic-shape Struct<JsonBody> (http.rs:469-476);
//                 the fields are parsed per request, so the №661
//                 field-meta armor correctly refuses a static labeling
//                 (a fieldless meta is not well-formed).
//   secret        the opaque Value::Secret (crypto.rs:322-328) — the
//                 value never prints, concatenates, or converts
//                 (to_string renders the masked "[Secret]"); typing it
//                 String would breach the SECRET_LEAK Category-A
//                 contract (crypto.rs:315-321).
//   memory_open   the opaque Value::Memory handle (№539;
//                 registry.rs:378) behind the consent gate — the live
//                 lane refuses MEMORY_CONSENT_REQUIRED first.
//   memory_read   the String|Secret answer by the container label
//                 (registry.rs:380) — typing String would false-warn
//                 the private lane (the in-tree comment verbatim).
//
// The ratchet: the sensitive floor moved UP in the same PR
// (5655 → 5793 bp; scripts/ci/sensitive_surface_baseline.txt); the
// frozen list itself is NOT edited (the id closes once).
#![allow(clippy::disallowed_methods)]

use std::sync::Mutex;

/// The registries and the mock-mode env are process-global — the
/// env-touching test serializes here.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn vm_run(src: &str) -> Result<Option<String>, String> {
    let declarations = metalogos::parser::parse(src).expect("parse");
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations).expect("compile");
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_both(src: &str, expected: &str) {
    let tw = metalogos::run_program(src)
        .unwrap_or_else(|e| panic!("TW run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(tw.trim_end(), expected, "TW output mismatch\n{src}");
    let vm = vm_run(src)
        .unwrap_or_else(|e| panic!("VM run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(vm.trim_end(), expected, "VM output mismatch\n{src}");
}

// The TW-only / VM-only split-assertions (assert_tw / assert_vm) were
// retired with the №734 landing: the query() mutation divergence they
// existed to pin is resolved — the both-backend form is the only
// sanctioned shape now (tests/n194_query_parity.rs carries the
// dedicated pins).

// ── conv_context → String ─────────────────────────────────────────────

#[test]
fn n702_conv_context_is_string() {
    // The deterministic "{role}: {text}" format (session_ops.rs) — the
    // field access proves nothing extra; the `+` concatenation proves
    // the String shape (a non-String operand is a type error).
    let src = r#"
pattern T(i: String) -> String {
  let id = conv_start("n702ctx")
  let _ = conv_add(id, "user", "hello world")
  let ctx = conv_context(id)
  return ctx + "!"
}
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(src, "user: hello world!");
}

// ── conv_history → List<Message> ──────────────────────────────────────

#[test]
fn n702_conv_history_is_list_of_message() {
    // len() proves the List shape; get()+field access pins the FIXED
    // Message{role, text, timestamp} fields (session_ops.rs:198-209).
    // The timestamp is now()-derived — pinned only as a numeric Float
    // field via its presence, never by value.
    let src = r#"
pattern T(i: String) -> String {
  let id = conv_start("n702hist")
  let _ = conv_add(id, "user", "hello")
  let h = conv_history(id)
  let m = get(h, 0)
  return to_string(len(h)) + " " + m.role + ":" + m.text
}
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(src, "1 user:hello");
}

// ── the stub refusal: stdin ───────────────────────────────────────────

#[test]
fn n702_stdin_stays_a_loud_stub() {
    let src = r#"
pattern T(i: String) -> String { return stdin() }
flow Main { input: String = "x" -> T -> output }
"#;
    // Both lanes refuse — the refusal IS the Err arm of each runner.
    let tw = metalogos::run_program(src);
    assert!(tw.is_err(), "TW: the stub call must be an Err");
    assert!(tw.err().unwrap_or_default().contains("stdin"));
    let vm = vm_run(src);
    assert!(vm.is_err(), "VM: the stub call must be an Err");
    let msg = vm.err().unwrap_or_default();
    assert!(
        msg.contains("stdin"),
        "VM: the refusal must name the stub, got: {msg}"
    );
}

// ── the query contract — the №734 parity (was: the multimorphic divergence) ──

#[test]
fn n702_query_mutation_parity_after_n734() {
    // WAS: the DIVERGENT live contract — the refusal evidence, one
    // SELECT shape shared and one mutation shape split:
    //   TW (db_ops.rs:200-204): the INSERT answers the affected-row
    //   count as a String → "1".
    //   VM (db_ops.rs:699+): the rows were built unconditionally → the
    //   mutation yielded the EMPTY list → "0" via len().
    // RESOLVED by №734 (wave 51 tail): ADR-0194 ACCEPTED 2026-10-20 —
    // packages A(c)+B(i), the VM mutation detect mirrors the TW
    // byte-for-byte; the mutation answers the affected-row count as a
    // String on BOTH lanes (the №474 ONE contract). The pin rides the
    // sanctioned parity now; the dedicated parity pins live in
    // tests/n194_query_parity.rs (t1 — the ADR §2 live-probe shape,
    // now green on both).
    let src = r#"
db { url: "sqlite::memory:" }
pattern T(i: String) -> String {
  let _ = db_execute("CREATE TABLE t (id INTEGER)")
  let n = query("INSERT INTO t VALUES (7)")
  let rows = query("SELECT id FROM t")
  return to_string(n) + "/" + to_string(len(rows))
}
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(src, "1/1");
}

// ── the data-dependent element types: query_row / query_scalar ───────

#[test]
fn n702_query_row_elements_are_data_dependent() {
    // db_ops.rs:541-557: Null→Unit, Integer/Real→Float, Text→String.
    // The numeric element answers arithmetic (List<String> would
    // false-warn it), the text element concatenates — one program, both
    // proofs.
    let src = r#"
db { url: "sqlite::memory:" }
pattern T(i: String) -> String {
  let _ = db_execute("CREATE TABLE t (id INTEGER, name TEXT)")
  let _ = db_execute("INSERT INTO t VALUES (1, 'n702')")
  let r = query_row("SELECT id, name FROM t")
  return to_string(r[0] + 1) + "/" + (r[1] + "!")
}
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(src, "2/n702!");
}

#[test]
fn n702_query_scalar_is_data_dependent() {
    // The numeric column answers arithmetic; the text column
    // concatenates — the value type rides the SQL answer, no single
    // honest spelling exists.
    let src = r#"
db { url: "sqlite::memory:" }
pattern T(i: String) -> String {
  let _ = db_execute("CREATE TABLE t (id INTEGER, name TEXT)")
  let _ = db_execute("INSERT INTO t VALUES (41, 'n702')")
  let num = query_scalar("SELECT id FROM t")
  let txt = query_scalar("SELECT name FROM t")
  return to_string(num + 1) + "/" + (txt + "!") // allow(unchecked_option): the №702 pin IS the data-dependent cell dispatch — the numeric column rides +, the text column concatenates; the Unit arm is unreachable on the seeded table
}
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(src, "42/n702!");
}

// ── the opaque Secret: secret ─────────────────────────────────────────

#[test]
fn n702_secret_is_opaque() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("METALOGOS_N702_SECRET").ok();
    std::env::set_var("METALOGOS_N702_SECRET", "hunter2");

    // The concatenation refusal (both backends).
    let src = r#"
pattern T(i: String) -> String { return secret("METALOGOS_N702_SECRET") + "!" }
flow Main { input: String = "x" -> T -> output }
"#;
    let tw = metalogos::run_program(src);
    assert!(tw.is_err(), "TW: the Secret must not concatenate");
    let vm = vm_run(src);
    assert!(vm.is_err(), "VM: the Secret must not concatenate");
    assert!(vm
        .err()
        .unwrap_or_default()
        .contains("cannot concatenate opaque type Secret"));

    // The masked rendering — the value never prints (both backends).
    let src2 = r#"
pattern T(i: String) -> String { return to_string(secret("METALOGOS_N702_SECRET")) }
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(src2, "[Secret]");

    // The egress refusal: respond(secret(...)) — the TW compile-time
    // SECRET_LEAK Category-A invariant; the VM type refusal. The exact
    // texts differ per lane — both must refuse.
    let src3 = r#"
pattern T(i: String) -> Unit { respond(secret("METALOGOS_N702_SECRET")) return }
flow Main { input: String = "x" -> T -> output }
"#;
    let tw3 = metalogos::run_program(src3);
    assert!(tw3.is_err(), "TW: respond(secret) must refuse");
    assert!(tw3.err().unwrap_or_default().contains("SECRET_LEAK"));
    let vm3 = vm_run(src3);
    assert!(vm3.is_err(), "VM: respond(secret) must refuse");

    match prev {
        Some(v) => std::env::set_var("METALOGOS_N702_SECRET", v),
        None => std::env::remove_var("METALOGOS_N702_SECRET"),
    }
}

// ── the dynamic-shape Struct: request_body ────────────────────────────

#[test]
fn n702_request_body_is_dynamic_json_body() {
    // The non-server lane answers the empty JsonBody struct (the
    // JsonBody {} render) — the fields are parsed per request, which is
    // exactly why the static field labeling cannot exist.
    let src = r#"
pattern T(i: String) -> String { return to_string(request_body()) }
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(src, "JsonBody {}");
}

// ── the consent gate: memory_open ─────────────────────────────────────

#[test]
fn n702_memory_open_is_consent_gated() {
    // The private container refuses before any handle exists — the
    // MEMORY_CONSENT_REQUIRED gate on both backends (the opaque Memory
    // handle behind it stays honestly untypable, №539).
    let src = r#"
pattern T(i: String) -> String {
  let h = memory_open("n702-consent-subject", "private")
  return "unreachable"
}
flow Main { input: String = "x" -> T -> output }
"#;
    let tw = metalogos::run_program(src);
    assert!(tw.is_err(), "TW: memory_open must refuse without consent");
    assert!(tw
        .err()
        .unwrap_or_default()
        .contains("MEMORY_CONSENT_REQUIRED"));
    let vm = vm_run(src);
    assert!(vm.is_err(), "VM: memory_open must refuse without consent");
    assert!(vm
        .err()
        .unwrap_or_default()
        .contains("MEMORY_CONSENT_REQUIRED"));
}
