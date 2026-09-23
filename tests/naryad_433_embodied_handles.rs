// ── Наряд №355 (P1, feature/embodied — registry В5, Phase 5
//    «Embodied, sim-only», ADR-0159): the opaque-handle contracts ──────
//
// Corpus:
//   (1) the seven embodied handles are OPAQUE (the ADR-0114 pattern):
//       Display markers, type names, non-printable — no payload in
//       Value, no detail through any projection;
//   (2) the device gate: `embodied-sim` records only — an unknown
//       backend refuses (EMBODIED_BACKEND_UNKNOWN), a foreign class
//       refuses (EMBODIED_CLASS_MISMATCH) — the contour is sim-only;
//   (3) NO UNMONITORED ACTION (ADR-0159 §2.4.2): chunk_make without a
//       bounds formula refuses EMBODIED_UNBOUNDED, with one — passes;
//   (4) the trajectory guards: empty/over-capacity refuse loudly;
//   (5) TW/VM parity: the same source runs identically on both
//       backends (the shared BUILTIN_REGISTRY dispatch);
//   (6) the registry profile: the embodied-sim entries carry the В2
//       license classes and the honest PendingNo334 declaration.

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

// ── (1) The opaque invariants ───────────────────────────────────────────

#[test]
fn embodied_handles_are_opaque() {
    // Display: bare markers, no ids, no payload.
    let dev = metalogos::embodied::device_open("t", "embodied-mock-device", None).unwrap();
    assert_eq!(format!("{}", dev), "[Device]");
    let ws = metalogos::embodied::world_state(
        "t",
        match &dev {
            metalogos::interpreter::values::Value::Device(m) => m.get("id").unwrap(),
            _ => panic!("device"),
        },
    )
    .unwrap();
    assert_eq!(format!("{}", ws), "[WorldState]");
    // type names: the seven registry В5 names.
    assert_eq!(dev.type_name(), "Device");
    assert_eq!(ws.type_name(), "WorldState");
    // Non-printable: the ADR-0114 convention for ALL opaque handles.
    assert!(metalogos::interpreter::values::is_nonprintable(&dev));
    assert!(metalogos::interpreter::values::is_nonprintable(&ws));
    // The projection map carries NO payload — Device: {id, backend};
    // WorldState: {id, device, label, step} — four words, no state.
    match &ws {
        metalogos::interpreter::values::Value::WorldState(m) => {
            assert_eq!(m.get("label").map(String::as_str), Some("private"));
            assert!(m.contains_key("id"));
            assert!(m.contains_key("device"));
            assert!(m.contains_key("step"));
            assert_eq!(m.len(), 4, "the projection is metadata only");
        }
        _ => panic!("world state"),
    }
}

#[test]
fn pose_trajectory_goal_proof_are_opaque_markers() {
    let pose = metalogos::embodied::pose_make("t", 1.0, 2.0, 3.0, 0.5).unwrap();
    assert_eq!(format!("{}", pose), "[Pose]");
    assert_eq!(pose.type_name(), "Pose");
    let poses = vec![pose.clone()];
    let traj = metalogos::embodied::trajectory_make("t", &poses).unwrap();
    assert_eq!(format!("{}", traj), "[Trajectory]");
    assert_eq!(traj.type_name(), "Trajectory");
    let goal = metalogos::embodied::goal_make("t", "eventually(at(goal))").unwrap();
    assert_eq!(format!("{}", goal), "[GoalPredicate]");
    assert_eq!(goal.type_name(), "GoalPredicate");
    assert!(metalogos::interpreter::values::is_nonprintable(&pose));
    assert!(metalogos::interpreter::values::is_nonprintable(&traj));
    assert!(metalogos::interpreter::values::is_nonprintable(&goal));
    // The numeric payload NEVER enters Value — the Pose projection is
    // {id, frame} only (the ADR-0114 Reflex rationale).
    match &pose {
        metalogos::interpreter::values::Value::Pose(m) => {
            assert_eq!(m.len(), 2, "id + frame — no coordinates in Value");
            assert_eq!(m.get("frame").map(String::as_str), Some("world"));
        }
        _ => panic!("pose"),
    }
}

// ── (2) The device gate: embodied-sim records only ──────────────────────

#[test]
fn device_open_refuses_unknown_backend() {
    let err = metalogos::embodied::device_open("device_open", "no-such-backend", None)
        .unwrap_err();
    assert!(
        err.starts_with("[EMBODIED_BACKEND_UNKNOWN]"),
        "typed stamp expected, got: {}",
        err
    );
}

#[test]
fn device_open_refuses_foreign_class() {
    // chatterbox is a real TTS registry record — a device over it is a
    // category error, not a degraded mode (the contour is sim-only).
    let err = metalogos::embodied::device_open("device_open", "chatterbox", None).unwrap_err();
    assert!(
        err.starts_with("[EMBODIED_CLASS_MISMATCH]"),
        "typed stamp expected, got: {}",
        err
    );
    assert!(err.contains("tts"), "the actual class names in the message");
}

#[test]
fn device_open_accepts_both_sim_records_and_validates_bounds() {
    for backend in ["embodied-sim-kinematic", "embodied-mock-device"] {
        let dev = metalogos::embodied::device_open("t", backend, None).expect(backend);
        assert_eq!(dev.type_name(), "Device");
    }
    // An empty bounds formula is a loud shape refusal (no silent
    // "unbounded-but-open" state).
    let err = metalogos::embodied::device_open("t", "embodied-mock-device", Some("   "))
        .unwrap_err();
    assert!(err.contains("non-empty"), "got: {}", err);
}

// ── (3) No unmonitored action (ADR-0159 §2.4.2) ─────────────────────────

#[test]
fn chunk_without_bounds_refuses_fail_closed() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dev = metalogos::embodied::device_open("t", "embodied-mock-device", None).unwrap();
    let dev_id = match &dev {
        metalogos::interpreter::values::Value::Device(m) => m.get("id").unwrap().clone(),
        _ => panic!("device"),
    };
    let pose = metalogos::embodied::pose_make("t", 0.0, 0.0, 0.0, 0.0).unwrap();
    let traj =
        metalogos::embodied::trajectory_make("t", std::slice::from_ref(&pose)).unwrap();
    let err = metalogos::embodied::chunk_make("chunk_make", &dev_id, traj_id(&traj), None)
        .unwrap_err();
    assert!(
        err.starts_with("[EMBODIED_UNBOUNDED]"),
        "the typed refusal expected, got: {}",
        err
    );
    assert!(err.contains("ADR-0159"), "the invariant's anchor names");
    // The refusal is itself an audit record (no silent refusal — the
    // №428 posture).
    let records = metalogos::ledger::all_records().unwrap_or_default();
    assert!(
        records
            .iter()
            .any(|r| r.action == "embodied.chunk_denied" && r.actor == dev_id),
        "chunk_denied must be a ledger record"
    );
}

#[test]
fn chunk_with_bounds_passes() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dev = metalogos::embodied::device_open("t", "embodied-mock-device", None).unwrap();
    let dev_id = handle_id(&dev, "id");
    let dev2 =
        metalogos::embodied::bounds_attach("t", &dev_id, "always(|pose.velocity| <= v_max)")
            .unwrap();
    let dev2_id = handle_id(&dev2, "id");
    let pose = metalogos::embodied::pose_make("t", 1.0, 2.0, 3.0, 0.25).unwrap();
    let traj = metalogos::embodied::trajectory_make("t", std::slice::from_ref(&pose)).unwrap();
    let goal = metalogos::embodied::goal_make("t", "eventually(at(goal))").unwrap();
    let chunk = metalogos::embodied::chunk_make(
        "t",
        &dev2_id,
        traj_id(&traj),
        Some(goal_id(&goal)),
    )
    .expect("a bounded device makes chunks");
    assert_eq!(chunk.type_name(), "ActionChunk");
    assert_eq!(format!("{}", chunk), "[ActionChunk]");
    // The chunk_make leg is a ledger record against the device.
    let chunk_id = handle_id(&chunk, "id");
    let records = metalogos::ledger::all_records().unwrap_or_default();
    assert!(
        records
            .iter()
            .any(|r| r.action == "embodied.chunk_make" && r.actor == chunk_id),
        "chunk_make must be a ledger record (the embodied.* family)"
    );
}

// ── (4) The trajectory guards ───────────────────────────────────────────

#[test]
fn trajectory_refuses_empty_and_over_capacity() {
    let err = metalogos::embodied::trajectory_make("t", &[]).unwrap_err();
    assert!(err.contains("empty"), "got: {}", err);
    let too_many: Vec<metalogos::interpreter::values::Value> = (0..1025)
        .map(|i| {
            metalogos::embodied::pose_make("t", i as f64, 0.0, 0.0, 0.0).unwrap()
        })
        .collect();
    let err = metalogos::embodied::trajectory_make("t", &too_many).unwrap_err();
    assert!(
        err.contains("1025") && err.contains("1024"),
        "the capacity guard names both numbers: {}",
        err
    );
}

#[test]
fn pose_refuses_non_finite_coordinates() {
    let err = metalogos::embodied::pose_make("t", f64::NAN, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("finite"), "got: {}", err);
    let err = metalogos::embodied::pose_make("t", 0.0, f64::INFINITY, 0.0, 0.0).unwrap_err();
    assert!(err.contains("finite"), "got: {}", err);
}

// ── (5) TW/VM parity of the language surface ────────────────────────────

const PARITY_SOURCE: &str = r#"
pattern EmbodiedParity(_tick: String) -> String {
  let dev = device_open("embodied-sim-kinematic")
  let dev2 = bounds_attach(dev, "always(|pose.velocity| <= v_max)")
  let ws = world_state(dev2)
  let p1 = pose_make(0.0, 0.0, 0.0, 0.0)
  let traj = trajectory_make([p1])
  let chunk = chunk_make(dev2, traj)
  let proof = proof_seal(ws, chunk)
  let rep = proof_verify(proof)
  return type_of(dev2) + "|" + type_of(ws) + "|" + type_of(chunk) + "|"
       + type_of(proof) + "|" + to_string(rep.valid) + "|" + rep.verdict
}
flow Main { input: String = "tick" -> EmbodiedParity -> output }
"#;

#[test]
fn embodied_surface_runs_identically_on_both_backends() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let base = base_dir();
    let tw = run_tw(PARITY_SOURCE, &base).expect("TW run");
    let vm = run_vm(PARITY_SOURCE, &base).expect("VM run");
    let tw = tw.unwrap_or_default();
    let vm = vm.unwrap_or_default();
    assert_eq!(
        tw, vm,
        "the embodied surface must be TW/VM-identical (the shared registry dispatch)"
    );
    assert_eq!(
        tw, "Device|WorldState|ActionChunk|Proof|true|pending",
        "the deterministic stage-A contract"
    );
}

#[test]
fn world_state_materialization_refusal_is_parity_typed() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let source = r#"
pattern WsRefusal(_tick: String) -> String {
  let dev = device_open("embodied-mock-device")
  let ws = world_state(dev)
  let r = try to_string(ws)
  return to_string(r.ok) + "|" + type_of(ws)
}
flow Main { input: String = "tick" -> WsRefusal -> output }
"#;
    let base = base_dir();
    let tw = run_tw(source, &base).expect("TW run");
    let vm = run_vm(source, &base).expect("VM run");
    assert_eq!(tw, vm, "the refusal parity");
    assert_eq!(
        tw.unwrap_or_default(),
        "false|WorldState",
        "to_string refused (ok=false), the type is intact"
    );
}

// ── (6) The registry profile (В2) ───────────────────────────────────────

#[test]
fn embodied_sim_registry_profile() {
    for name in ["embodied-sim-kinematic", "embodied-mock-device"] {
        let e = metalogos::backends::find_by_name(name)
            .unwrap_or_else(|| panic!("{} in the registry", name));
        assert_eq!(e.class, metalogos::backends::BackendClass::EmbodiedSim);
        assert_eq!(e.class.as_str(), "embodied-sim");
        assert_eq!(
            metalogos::backends::BackendClass::parse("embodied-sim"),
            Some(metalogos::backends::BackendClass::EmbodiedSim)
        );
        // The license profile: in-tree records — the repo license (osi).
        assert!(matches!(e.license, metalogos::backends::LicenseClass::Osi));
        assert!(e.license_note.contains("in-tree"), "{}", e.license_note);
        // The honest pin declaration: nothing to fetch or pin.
        assert!(matches!(
            e.pin,
            metalogos::backends::ShaPin::PendingNo334
        ));
    }
}

// ── helpers ─────────────────────────────────────────────────────────────

fn handle_id(v: &metalogos::interpreter::values::Value, key: &str) -> String {
    match v {
        metalogos::interpreter::values::Value::Device(m)
        | metalogos::interpreter::values::Value::WorldState(m)
        | metalogos::interpreter::values::Value::ActionChunk(m)
        | metalogos::interpreter::values::Value::Pose(m)
        | metalogos::interpreter::values::Value::Trajectory(m)
        | metalogos::interpreter::values::Value::GoalPredicate(m)
        | metalogos::interpreter::values::Value::Proof(m) => m
            .get(key)
            .cloned()
            .unwrap_or_else(|| panic!("handle missing {}", key)),
        other => panic!("not an embodied handle: {}", other.type_name()),
    }
}

fn traj_id(v: &metalogos::interpreter::values::Value) -> &str {
    match v {
        metalogos::interpreter::values::Value::Trajectory(m) => {
            m.get("id").map(String::as_str).unwrap_or("")
        }
        _ => "",
    }
}

fn goal_id(v: &metalogos::interpreter::values::Value) -> &str {
    match v {
        metalogos::interpreter::values::Value::GoalPredicate(m) => {
            m.get("id").map(String::as_str).unwrap_or("")
        }
        _ => "",
    }
}
