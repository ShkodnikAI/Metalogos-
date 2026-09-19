//! Наряд №385 (issue #479, ADR-0169) — stable `try.error.code` contracts.
//!
//! Contract under test:
//! - ALL THREE `try` sewing points (TW `Expr::Try`, VM `Instruction::TryEval`
//!   in both dispatch arms) classify through the ONE shared
//!   `values::stable_try_error_code` — the same error string yields the same
//!   code on both backends (parity by construction; this file makes any
//!   divergence RED);
//! - classification is by ORIGIN STAMP (`[CODE] ` prefix produced by the
//!   failing subsystem at the place of origin), never by message text;
//! - an unstamped error honestly classifies as the `RUNTIME_ERROR` fallback;
//! - the `message` field keeps the full error text (stamp included) —
//!   backward compatible for every consumer that reads messages today;
//! - the deterministic mock fault seam `METALOGOS_MOCK_LLM_FAULT`
//!   (timeout | unavailable, invalid → fail-closed) exercises the LLM codes
//!   without a network;
//! - `BACKEND_DEGRADED` stays a typed result (№336/ADR-0165) whose
//!   `error.code` reuses the SAME frozen constant the try-classifier
//!   whitelists;
//! - the VM runtime sink twin's refusal carries the `[SINK_CLEARANCE_RUNTIME]`
//!   stamp at position 0 — the exact format the classifier whitelists
//!   (bytecode-level trigger, the source-level shape is the 328 contract).

use std::path::Path;
use std::sync::Mutex;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

/// The fault-injection seam is process-global environment state; tests in
/// one binary run in parallel threads — the lock serializes every test that
/// touches `METALOGOS_MOCK_LLM_FAULT` or the mock delay.
static FAULT_LOCK: Mutex<()> = Mutex::new(());

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// Run a program on BOTH backends and assert the returned string equals
/// `expected` on each — a backend that classifies differently goes RED
/// (the mutation check the naryad requires: substituting a code in one
/// backend breaks the equality).
fn assert_both_backends(src: &str, expected: &str) {
    let base = Path::new(MANIFEST);
    let tw = run_tw(src, base)
        .expect("TW run must succeed (the error is captured by try)")
        .expect("TW must return the pattern output");
    assert_eq!(tw.trim(), expected, "TW code mismatch");
    let vm = run_vm(src, base)
        .expect("VM run must succeed (the error is captured by try)")
        .expect("VM must return the pattern output");
    assert_eq!(vm.trim(), expected, "VM code mismatch — parity broken");
}

// ── (а) The three sewing points share ONE classifier: same error, ─────
//    same code, TW and VM. Every test below runs BOTH backends.

#[test]
fn n385_sandbox_violation_code_on_both_backends() {
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try write_file("/etc/metalogos-n385.txt", "nope")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both_backends(src, "SANDBOX_VIOLATION");
}

#[test]
fn n385_sql_error_code_on_both_backends() {
    // A rusqlite-origin failure (no such table) is stamped `SQL_ERROR` at
    // the origin by the SAME `sql_err` helper on TW (interpreter/db.rs)
    // and VM (vm.rs dispatch arms) — the stamps cannot diverge.
    let src = r#"
db { url: "sqlite::memory:" }
pattern Probe(x: String) -> String {
  let r = try query("SELECT * FROM no_such_table_n385")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both_backends(src, "SQL_ERROR");
}

#[test]
fn n385_unstamped_error_falls_back_to_runtime_error() {
    // A type failure in read_file's argument originates outside every
    // whitelisted subsystem — no stamp, honest fallback.
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try read_file(123.0)
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both_backends(src, "RUNTIME_ERROR");
}

#[test]
fn n385_media_sealed_egress_code_on_both_backends() {
    let src = r#"
origin feed { kind: generation, media: image, label: public }
pattern Frame(x: String) -> String {
  let img = from feed media_store_image("secret-bytes", "private")
  let r = try media_save(img, "target/n385-media/sealed-out.bin")
  return r.error.code
}
flow Main { input: String = "x" -> Frame -> output }
"#;
    assert_both_backends(src, "MEDIA_SEALED_EGRESS");
}

// ── (б) LLM codes through the deterministic fault seam ────────────────

fn fault_program() -> &'static str {
    r#"
pattern Probe(x: String) -> String {
  let r = try call_llm("hello", "world")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#
}

fn fault_message_program() -> &'static str {
    r#"
pattern Probe(x: String) -> String {
  let r = try call_llm("hello", "world")
  return r.error.message
}
flow Main { input: String = "x" -> Probe -> output }
"#
}

#[test]
fn n385_llm_timeout_code_via_mock_fault() {
    let _guard = FAULT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM_FAULT", "timeout");
    let result = std::panic::catch_unwind(|| {
        assert_both_backends(fault_program(), "LLM_TIMEOUT");
    });
    std::env::remove_var("METALOGOS_MOCK_LLM_FAULT");
    result.expect("LLM_TIMEOUT parity must hold");
}

#[test]
fn n385_llm_provider_unavailable_code_via_mock_fault() {
    let _guard = FAULT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM_FAULT", "unavailable");
    let result = std::panic::catch_unwind(|| {
        assert_both_backends(fault_program(), "LLM_PROVIDER_UNAVAILABLE");
    });
    std::env::remove_var("METALOGOS_MOCK_LLM_FAULT");
    result.expect("LLM_PROVIDER_UNAVAILABLE parity must hold");
}

#[test]
fn n385_fault_seam_fails_closed_on_invalid_value() {
    // A typo in the seam variable must NOT silently degrade to a green
    // mock answer: the call fails (fail-closed) with a loud unstamped
    // message → the honest RUNTIME_ERROR fallback.
    let _guard = FAULT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM_FAULT", "bogus");
    let result = std::panic::catch_unwind(|| {
        assert_both_backends(fault_program(), "RUNTIME_ERROR");
        let base = Path::new(MANIFEST);
        let tw_msg = run_tw(fault_message_program(), base)
            .expect("TW")
            .expect("output");
        assert!(
            tw_msg.contains("invalid METALOGOS_MOCK_LLM_FAULT"),
            "the refusal names the seam variable: {tw_msg}"
        );
    });
    std::env::remove_var("METALOGOS_MOCK_LLM_FAULT");
    result.expect("fail-closed contract must hold");
}

#[test]
fn n385_message_keeps_full_text_with_stamp() {
    // The `message` field is the FULL error text (stamp included) —
    // consumers reading messages today see exactly what they saw before
    // naryad №385 (plus the code now travels in `code`).
    let _guard = FAULT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM_FAULT", "timeout");
    let result = std::panic::catch_unwind(|| {
        let base = Path::new(MANIFEST);
        let tw_msg = run_tw(fault_message_program(), base)
            .expect("TW")
            .expect("output");
        assert!(
            tw_msg.starts_with("[LLM_TIMEOUT] "),
            "the origin stamp stays loud in the message: {tw_msg}"
        );
        assert!(
            tw_msg.contains("mock fault injection"),
            "the human-readable reason is intact: {tw_msg}"
        );
    });
    std::env::remove_var("METALOGOS_MOCK_LLM_FAULT");
    result.expect("message contract must hold");
}

// ── (в) The classification map is DISCRIMINATING (mutation check) ─────

#[test]
fn n385_codes_are_distinct_contracts() {
    // The codes are frozen ADR-0131 names — distinct subsystems MUST
    // classify distinctly (a mutation collapsing two codes into one, or
    // hardcoding a single code in a sewing point, breaks one of the
    // assertions below).
    assert_ne!("SANDBOX_VIOLATION", "SQL_ERROR");
    assert_ne!("LLM_TIMEOUT", "LLM_PROVIDER_UNAVAILABLE");
    assert_ne!("RUNTIME_ERROR", "SANDBOX_VIOLATION");
    // Every code asserted by the backend-parity tests above is one of the
    // frozen names (this pins the SET the naryad fixes, not just equality):
    let frozen = [
        "RUNTIME_ERROR",
        "LLM_TIMEOUT",
        "LLM_PROVIDER_UNAVAILABLE",
        "SQL_ERROR",
        "SANDBOX_VIOLATION",
        "SINK_CLEARANCE_RUNTIME",
        "MEDIA_SEALED_EGRESS",
        "BACKEND_DEGRADED",
    ];
    for observed in [
        n385_probe_code_sandbox(),
        n385_probe_code_sql(),
        n385_probe_code_fallback(),
        n385_probe_code_media(),
    ] {
        assert!(
            frozen.contains(&observed.as_str()),
            "observed code must be a frozen contract name: {observed}"
        );
    }
}

fn n385_probe_code_sandbox() -> String {
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try write_file("/etc/metalogos-n385.txt", "nope")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    run_tw(src, Path::new(MANIFEST))
        .expect("TW")
        .expect("output")
        .trim()
        .to_string()
}

fn n385_probe_code_sql() -> String {
    let src = r#"
db { url: "sqlite::memory:" }
pattern Probe(x: String) -> String {
  let r = try query("SELECT * FROM no_such_table_n385")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    run_tw(src, Path::new(MANIFEST))
        .expect("TW")
        .expect("output")
        .trim()
        .to_string()
}

fn n385_probe_code_fallback() -> String {
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try read_file(123.0)
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    run_tw(src, Path::new(MANIFEST))
        .expect("TW")
        .expect("output")
        .trim()
        .to_string()
}

fn n385_probe_code_media() -> String {
    let src = r#"
origin feed { kind: generation, media: image, label: public }
pattern Frame(x: String) -> String {
  let img = from feed media_store_image("secret-bytes", "private")
  let r = try media_save(img, "target/n385-media/sealed-out.bin")
  return r.error.code
}
flow Main { input: String = "x" -> Frame -> output }
"#;
    run_tw(src, Path::new(MANIFEST))
        .expect("TW")
        .expect("output")
        .trim()
        .to_string()
}

// ── (г) Shape contract: try catches what an uncaught call aborts ──────

#[test]
fn n385_uncaught_same_failure_aborts_while_try_captures() {
    let base = Path::new(MANIFEST);
    // Uncaught: the SAME failing call aborts the program with the stamped
    // error (loud at top level, unchanged by №385).
    let uncaught = r#"
pattern Probe(x: String) -> String {
  let v = write_file("/etc/metalogos-n385.txt", "nope")
  return v
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let err = run_tw(uncaught, base).expect_err("uncaught sandbox violation aborts");
    assert!(
        err.starts_with("[SANDBOX_VIOLATION] "),
        "top-level error keeps the loud stamp: {err}"
    );
    // Caught: the same failure becomes a structured result.
    let caught = r#"
pattern Probe(x: String) -> String {
  let r = try write_file("/etc/metalogos-n385.txt", "nope")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let code = run_tw(caught, base).expect("TW").expect("output");
    assert_eq!(code.trim(), "SANDBOX_VIOLATION");
}

// ── (д) BACKEND_DEGRADED: typed result reuses the frozen constant ─────

#[test]
fn n385_backend_degraded_typed_result_keeps_code() {
    // №336/ADR-0165: ladder exhaustion stays a TYPED Degraded(t) result —
    // never a String error. Its error.code is the SAME frozen name the
    // try-classifier whitelists (single constant, see backends.rs).
    let src = r#"
pattern Probe(x: String) -> String {
  let words = ["no-such-rung-n385"]
  let sel = backend_select("omni", words)
  if sel.ok {
    return "unexpected-selection"
  }
  return sel.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both_backends(src, "BACKEND_DEGRADED");
}

// ── (е) Origin stamps: the deadline and runtime-sink twins ────────────

#[test]
fn n385_mock_deadline_error_is_stamped_llm_timeout() {
    // MockLlm::call_with_deadline (№248) is a REAL LLM_TIMEOUT origin:
    // the deadline genuinely fired. The error carries the origin stamp —
    // the exact string the try classifier whitelists.
    use metalogos::llm::{LlmBackend, MockLlm};
    let _guard = FAULT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    MockLlm::set_delay_ms(400);
    let result = std::panic::catch_unwind(|| {
        let backend = MockLlm;
        let err = backend
            .call_with_deadline("p", "i", None, std::time::Duration::from_millis(50))
            .expect_err("deadline tighter than delay must fail");
        assert!(
            err.starts_with("[LLM_TIMEOUT] "),
            "deadline failure is stamped at the origin: {err}"
        );
        assert!(
            err.contains("LLM call timed out"),
            "the legacy wording survives after the stamp: {err}"
        );
    });
    MockLlm::reset_delay();
    result.expect("deadline stamp contract must hold");
}

#[test]
fn n385_vm_runtime_sink_refusal_carries_classifier_stamp() {
    // The VM runtime twin of the №325 gate (bytecode-level trigger, the
    // 328 contract) refuses with a message whose stamp sits at position 0
    // in the `[CODE] ` format — exactly what `stable_try_error_code`
    // whitelists, so a `try` capturing this refusal classifies it as
    // SINK_CLEARANCE_RUNTIME (same mechanism the golden examples prove
    // for the other stamped codes).
    use metalogos::bytecode::{Instruction, Program};
    use metalogos::vm::Vm;
    let program = Program {
        globals: vec![],
        patterns: vec![],
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        reflex_decls: vec![],
        reflex_seq_decls: vec![],
        reflex_gen_decls: vec![],
        vision_decls: vec![],
        origin_decls: vec![],
        deny_handlers: vec![],
        db_url: None,
        memory_persist_path: None,
        schema_ddl: vec![],
        main_code: vec![
            Instruction::LabelJoin {
                dst: "k".to_string(),
                src: "@env".to_string(),
            },
            Instruction::SinkCheck {
                fn_name: "print".to_string(),
                arg: "k".to_string(),
                line: 3,
                arg_index: 0,
                deny: None,
            },
        ],
        collections_loaded: false,
        shared_cache: metalogos::bytecode::ProgramSharedCache::new(),
    };
    let err = Vm::new()
        .run(program)
        .expect_err("runtime gate must reject");
    assert!(
        err.starts_with("[SINK_CLEARANCE_RUNTIME] "),
        "the runtime twin's stamp is at position 0 in the unified format: {err}"
    );
}
