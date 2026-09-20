//! Naryad №413 (issue #558, ADR-0169 §3.1 extension) — try-codes for the
//! cron mechanics and the MCP contour.
//!
//! Contract under test (incremental per the audit rule: only codes with a
//! real consumer, the frozen №385 set is EXTENDED, never renamed):
//! - cron/reminder mechanics failures travel the error channel stamped
//!   `CRON_JOB_FAILED` at the place that knows what failed — the same
//!   error string classifies identically on BOTH backends (the one shared
//!   classifier, parity by construction);
//! - the MCP contour's EXISTING origin markers (`MCP_SPAWN_FAILED`,
//!   `MCP_TIMEOUT`, `MCP_IO_ERROR`, `MCP_TOOL_ERROR`, `MCP_TOOL_NOT_FOUND`,
//!   `MCP_PROTOCOL_ERROR`, `MCP_NOT_ALLOWLISTED`) are now classifier-visible
//!   (the naryad whitelists the finer taxonomy instead of adding a coarser
//!   duplicate; the audit's `MCP_TOOL_FAILED` name is recorded as such);
//! - unstamped paths still classify as the honest `RUNTIME_ERROR`
//!   fallback (the №385 contract is untouched);
//! - the wrapper layer (`with_mcp_server`) no longer buries a position-0
//!   stamp mid-message (`wrap_error_preserving_code`).
//!
//! Mutation verification (№382 protocol): `scripts/mutation_verify_413.sh`
//! — neutering the cron stamp helper makes the cron tests fall; dropping
//! the MCP entries from `ORIGIN_STAMPED_CODES` makes the MCP tests fall.

use std::path::Path;
use std::sync::Mutex;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

/// The MCP exec gate is process-global environment state (№253); tests in
/// one binary run in parallel threads — serialize every env mutation.
static ENV_LOCK: Mutex<()> = Mutex::new(());

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

/// Run on BOTH backends and assert the returned string equals `expected`
/// on each — a backend that classifies differently goes RED.
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

// ── (а) cron mechanics → CRON_JOB_FAILED, both backends ────────────────

#[test]
fn n413_cron_invalid_expression_code_on_both_backends() {
    // The 5-field cron contract is the cron subsystem's own refusal —
    // stamped `CRON_JOB_FAILED` at the origin (the builtin boundary).
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try cron_add("not-a-cron", "some_pattern")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both_backends(src, "CRON_JOB_FAILED");
}

#[test]
fn n413_cron_negative_interval_code_on_both_backends() {
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try remind_recurring("check the ledger", -5.0)
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both_backends(src, "CRON_JOB_FAILED");
}

#[test]
fn n413_cron_arg_type_refusal_code_on_both_backends() {
    // Even the arg/type refusals of the cron family carry the subsystem
    // stamp (the whole mechanics surface is stamped at its boundary).
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try cron_add(123.0, "some_pattern")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both_backends(src, "CRON_JOB_FAILED");
}

#[test]
fn n413_cron_happy_path_is_not_an_error() {
    // A valid registration must NOT be an error — the stamp is for
    // failures, never for the success path.
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try cron_add("0 9 * * 1-5", "some_pattern")
  return r.ok
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both_backends(src, "true");
}

// ── (б) the MCP contour → the finer markers + the transport fallback ───

#[test]
fn n413_mcp_spawn_failure_code_on_both_backends() {
    // A server process that cannot spawn carries the existing
    // `[MCP_SPAWN_FAILED]` origin stamp (the transport class) — now
    // classifier-visible. Requires the exec gate (№253).
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    // The allowlist must not refuse first — unset it for the spawn probe.
    let saved_allowlist = std::env::var("METALOGOS_MCP_ALLOWLIST").ok();
    std::env::remove_var("METALOGOS_MCP_ALLOWLIST");
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try mcp_call("/nonexistent/n413-no-such-server", [], "probe_tool", "{}")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let result = (|| -> Result<(String, String), String> {
        let base = Path::new(MANIFEST);
        let tw = run_tw(src, base)?.ok_or("TW output missing")?;
        let vm = run_vm(src, base)?.ok_or("VM output missing")?;
        Ok((tw, vm))
    })();
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    match saved_allowlist {
        Some(v) => std::env::set_var("METALOGOS_MCP_ALLOWLIST", v),
        None => std::env::remove_var("METALOGOS_MCP_ALLOWLIST"),
    }
    let (tw, vm) = result.expect("both backends must run");
    assert_eq!(tw.trim(), "MCP_SPAWN_FAILED", "TW transport code");
    assert_eq!(
        vm.trim(),
        "MCP_SPAWN_FAILED",
        "VM transport code — parity broken"
    );
}

#[test]
fn n413_mcp_allowlist_refusal_code() {
    // The policy refusal carries the existing `MCP_NOT_ALLOWLISTED` origin
    // marker — now classifier-visible (TW run is enough: the marker is
    // produced before any process spawn, and the classifier is shared).
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    std::env::set_var("METALOGOS_MCP_ALLOWLIST", "/some/other/server");
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try mcp_call("/nonexistent/n413-allowlist-probe", [], "probe_tool", "{}")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let base = Path::new(MANIFEST);
    let tw = run_tw(src, base).expect("TW run").expect("TW output");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    std::env::remove_var("METALOGOS_MCP_ALLOWLIST");
    assert_eq!(tw.trim(), "MCP_NOT_ALLOWLISTED", "policy refusal code");
}

// ── (в) the №385 fallback contract is untouched ────────────────────────

#[test]
fn n413_unstamped_error_still_falls_back_to_runtime_error() {
    // A type failure outside every whitelisted subsystem — no stamp,
    // honest fallback (the frozen set extension must not have widened
    // the fallback's territory).
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
fn n413_stamps_survive_the_wrapper_layer() {
    // The regression that motivated `wrap_error_preserving_code` in the
    // MCP contour: a failure AFTER the spawn travels through the
    // `with_mcp_server` wrapper, which used to prefix `mcp_call(): ` and
    // DEMOTE the origin stamp to mid-message prose (classification fell
    // to RUNTIME_ERROR). The message KEEPS the head text while the stamp
    // stays at position 0. The garbage fixture fails the handshake with
    // the existing `[MCP_PROTOCOL_ERROR]` origin marker.
    let base = Path::new(MANIFEST);
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    let saved_allowlist = std::env::var("METALOGOS_MCP_ALLOWLIST").ok();
    std::env::remove_var("METALOGOS_MCP_ALLOWLIST");
    let saved_garbage = std::env::var("MCP_FIXTURE_GARBAGE").ok();
    std::env::set_var("MCP_FIXTURE_GARBAGE", "1");
    let src = r#"
pattern Probe(x: String) -> String {
  let cmd = "python3 " + x
  let r = try mcp_call("python3", ["tests/fixtures/mcp_echo_server.py"], "echo", "{}")
  return r.error.code + "|" + r.error.message
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let tw = run_tw(src, base).expect("TW run").expect("TW output");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    std::env::remove_var("MCP_FIXTURE_GARBAGE");
    match (saved_allowlist, saved_garbage) {
        (Some(v), _) => std::env::set_var("METALOGOS_MCP_ALLOWLIST", v),
        (None, _) => std::env::remove_var("METALOGOS_MCP_ALLOWLIST"),
    }
    let (code, message) = match tw.split_once('|') {
        Some(pair) => pair,
        None => panic!("expected code|message, got: {}", tw),
    };
    assert_eq!(code, "MCP_PROTOCOL_ERROR", "handshake failure code");
    assert!(
        message.contains("mcp_call()"),
        "the wrapper head must stay in the message text: {}",
        message
    );
    assert!(
        message.trim_start().starts_with("[MCP_PROTOCOL_ERROR] "),
        "the stamp must sit at position 0 of the message: {}",
        message
    );
}
