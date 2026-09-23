// ── Наряд №355 (P1, feature/embodied — ADR-0159): the WorldState
//    private-by-default contract ───────────────────────────────────────
//
// The Phase-1 lattice leg (the №349 consistency): the world snapshot is
// PRIVATE state — its materialization outside the verified contour is a
// TYPED refusal (WORLD_STATE_PRIVATE, position-0 origin stamp — the
// №413 convention, try-branchable) AND an audit record
// (embodied.world_state_denied) on EVERY surface: print / to_string /
// json_encode. No silent egress AND no silent refusal (the №428
// posture). The remaining embodied handles project through their opaque
// Display markers — markers, never content.
//
// The corpus exercises the LANGUAGE surface end-to-end (both backends):
// the unwrapped call fails the run with the stamped error; the
// try-wrapped call branches on it (the office-policy shape).
//
// The leak-corpus boundary (loud): the corpus negatives are COMPILE-time
// classes; the WorldState refusal is a RUNTIME gate (the static
// WORLD_STATE_EGRESS check is the Фаза-1 lattice / monitor contour —
// the same posture as Memory<K> in №429, the limitations row). The
// runtime refusals are pinned HERE, with parity, not in examples/leak/.

use std::path::Path;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const BASE: &str = env!("CARGO_MANIFEST_DIR");

fn base_dir() -> std::path::PathBuf {
    std::path::Path::new(BASE).to_path_buf()
}

/// A program whose body is ONE materialization attempt over the world
/// snapshot (no try — the refusal must fail the run with the stamp).
fn refusal_source(call: &str) -> String {
    format!(
        r#"
pattern WsRefusal(_tick: String) -> String {{
  let dev = device_open("embodied-mock-device")
  let ws = world_state(dev)
  return {call}
}}
flow Main {{ input: String = "tick" -> WsRefusal -> output }}
"#
    )
}

fn world_state_denied_records() -> Vec<String> {
    metalogos::ledger::all_records()
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.action == "embodied.world_state_denied")
        .map(|r| r.actor)
        .collect()
}

// ── (1) The label is loud in the projection ─────────────────────────────

#[test]
fn world_state_projection_carries_the_private_label() {
    let dev = metalogos::embodied::device_open("t", "embodied-mock-device", None).unwrap();
    let dev_id = match &dev {
        metalogos::interpreter::values::Value::Device(m) => m.get("id").unwrap().clone(),
        _ => panic!("device"),
    };
    let ws = metalogos::embodied::world_state("t", &dev_id).unwrap();
    match &ws {
        metalogos::interpreter::values::Value::WorldState(m) => {
            assert_eq!(m.get("label").map(String::as_str), Some("private"));
            // The projection is metadata only — four words, no state.
            assert_eq!(m.len(), 4);
        }
        _ => panic!("world state"),
    }
    let id = match &ws {
        metalogos::interpreter::values::Value::WorldState(m) => m.get("id").unwrap().clone(),
        _ => panic!("world state"),
    };
    assert_eq!(
        metalogos::embodied::world_label(&id).as_deref(),
        Some("private")
    );
}

// ── (2) print — the typed refusal + the audit record, both backends ─────

#[test]
fn print_world_state_refuses_typed_and_audits_on_tw() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let err = run_tw(&refusal_source("print(ws)"), &base_dir()).unwrap_err();
    assert!(
        err.contains("[WORLD_STATE_PRIVATE]"),
        "the position-0 stamp expected, got: {}",
        err
    );
    assert!(
        err.contains("private-by-default"),
        "the contract wording: {}",
        err
    );
    assert!(
        !world_state_denied_records().is_empty(),
        "the print refusal must be an audit record"
    );
}

#[test]
fn print_world_state_refuses_typed_on_vm() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let err = run_vm(&refusal_source("print(ws)"), &base_dir()).unwrap_err();
    assert!(err.contains("[WORLD_STATE_PRIVATE]"), "got: {}", err);
}

// ── (3) to_string / json_encode — the same typed refusal ────────────────

#[test]
fn to_string_world_state_refuses_typed_on_both_backends() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let src = refusal_source("to_string(ws)");
    let tw_err = run_tw(&src, &base_dir()).unwrap_err();
    let vm_err = run_vm(&src, &base_dir()).unwrap_err();
    assert!(tw_err.contains("[WORLD_STATE_PRIVATE]"), "{}", tw_err);
    assert!(vm_err.contains("[WORLD_STATE_PRIVATE]"), "{}", vm_err);
    assert!(
        !world_state_denied_records().is_empty(),
        "the to_string refusal must be an audit record"
    );
}

#[test]
fn json_encode_world_state_refuses_before_serialization() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // json_encode is the one surface where the projection map would
    // become CONTENT — the refusal fires BEFORE serialization.
    let src = refusal_source("json_encode(ws)");
    let tw_err = run_tw(&src, &base_dir()).unwrap_err();
    let vm_err = run_vm(&src, &base_dir()).unwrap_err();
    assert!(tw_err.contains("[WORLD_STATE_PRIVATE]"), "{}", tw_err);
    assert!(vm_err.contains("[WORLD_STATE_PRIVATE]"), "{}", vm_err);
}

// ── (4) The other handles: opaque markers, never content ────────────────

#[test]
fn other_embodied_handles_project_markers_only() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let source = r#"
pattern Markers(_tick: String) -> String {
  let dev = device_open("embodied-mock-device")
  let s = to_string(dev)
  let j = json_encode(dev)
  return s + "@" + j
}
flow Main { input: String = "tick" -> Markers -> output }
"#;
    let base = base_dir();
    let tw = run_tw(source, &base).expect("TW run");
    let vm = run_vm(source, &base).expect("VM run");
    // The registry mints a fresh id per run (time+seq) — mask it before
    // the parity comparison; the STRUCTURE must be identical.
    let tw_out = mask_handle_ids(&tw.unwrap_or_default());
    let vm_out = mask_handle_ids(&vm.unwrap_or_default());
    assert_eq!(tw_out, vm_out, "the marker projection parity");
    let out = tw_out;
    let (marker, json) = out.split_once('@').expect("marker@json shape");
    assert_eq!(marker, "[Device]", "to_string = the opaque marker");
    assert!(
        json.contains("\"backend\":\"embodied-mock-device\""),
        "the metadata projection carries the backend: {}",
        json
    );
    assert!(
        json.contains("\"id\""),
        "the metadata projection carries the id: {}",
        json
    );
    assert!(
        !json.contains("telemetry") && !json.contains("weight"),
        "no state content: {}",
        json
    );
}

/// Mask the minted handle-id hex suffixes (`dev-<16hex>` etc.) so the
/// parity comparison sees the structure, not the per-run randomness.
fn mask_handle_ids(s: &str) -> String {
    let prefixes = ["dev", "wld", "bnd", "chk", "prf", "pos", "trj", "gol"];
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // A handle id = one of the prefixes + '-' + 16 hex chars; the
        // '-' lands at i while the prefix sits at the end of `out`.
        if bytes[i] == b'-' && prefixes.iter().any(|p| out.ends_with(p)) {
            let hex_start = i + 1;
            let hex_end = (hex_start + 16).min(bytes.len());
            if hex_end - hex_start == 16
                && bytes[hex_start..hex_end]
                    .iter()
                    .all(|c| c.is_ascii_hexdigit())
            {
                out.push_str("-<id>");
                i = hex_end;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

// ── (5) The try-branchable stamp, TW/VM parity ──────────────────────────

#[test]
fn refusal_stamp_branches_through_try_on_both_backends() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let source = r#"
pattern WsPolicy(_tick: String) -> String {
  let dev = device_open("embodied-mock-device")
  let ws = world_state(dev)
  let r = try print(ws)
  let s = try to_string(ws)
  return to_string(r.ok) + "|" + to_string(s.ok)
}
flow Main { input: String = "tick" -> WsPolicy -> output }
"#;
    let base = base_dir();
    let tw = run_tw(source, &base).expect("TW run");
    let vm = run_vm(source, &base).expect("VM run");
    assert_eq!(tw, vm, "the refusal parity");
    assert_eq!(
        tw.unwrap_or_default(),
        "false|false",
        "both materializations refuse through try"
    );
}
