// ── Naryad #355 (P1, feature/embodied — registry В5, Phase 5
//    «Embodied, sim-only», ADR-0159): the embodied builtins ────────────
//
// The Language surface of the embodied contour (state: src/embodied.rs).
// Arity/typing contract (registry SSOT — keep in sync with spec! rows):
//
//   device_open(backend, bounds?)          -> Device   opens a SIM device
//     over an `embodied-sim` registry record (ADR-0163 SSOT); unknown
//     backend — EMBODIED_BACKEND_UNKNOWN; a non-sim class —
//     EMBODIED_CLASS_MISMATCH (fail-closed; the contour is sim-only)
//   bounds_attach(device, formula)         -> Device   attaches (or
//     replaces) the bounds formula — the verbatim STL-style text,
//     stored opaque (digests only outside the contour)
//   device_state(device)                   -> Struct   {id, backend,
//     bounds_present, bounds_digest} — the introspection projection
//   world_state(device)                    -> WorldState  the deterministic
//     stage-A snapshot (step 0) — PRIVATE by default; print/to_string/
//     json_encode refuse it (WORLD_STATE_PRIVATE + audit)
//   pose_make(x, y, z, yaw)                -> Pose     the pose handle
//     (payload in the registry, the ADR-0114 opaque pattern)
//   trajectory_make(poses)                 -> Trajectory  over a List of
//     Pose handles; empty/over-capacity refuse loudly
//   goal_make(predicate)                   -> GoalPredicate  the verbatim
//     predicate (evaluated by the №356 monitor)
//   chunk_make(device, trajectory, goal?)  -> ActionChunk  WITHOUT a
//     bounds formula on the device profile — EMBODIED_UNBOUNDED
//     (ADR-0159 §2.4.2: no unmonitored action, fail-closed)
//   proof_seal(world, chunk, telemetry?)   -> Proof    the signed trace
//     {hash(world), chunk, bounds, telemetry-digest, verdict,
//     hash(sim)}; the stage-A verdict is PENDING by construction (the
//     monitor lands with №356 — no program-forged "satisfied")
//   proof_verify(proof)                    -> Struct   {valid, proof_id,
//     verdict, world_hash, chunk_id, bounds_id, backend, reasons} —
//     the CONTRACT FIELDS only (signature integrity, verdict
//     vocabulary, bounds/pin resolution); execution is №356

use super::core::expect_string_arg;
use crate::embodied;
use crate::interpreter::values::Value;

/// One finite-float argument (the pose coordinates). The language has
/// no Int — every number is Float, matched here explicitly (no soft
/// coercion: a wrong type is a loud argument error).
fn expect_float_arg(fn_name: &str, args: &[Value], idx: usize, name: &str) -> Result<f64, String> {
    match args.get(idx) {
        Some(Value::Float(f)) => Ok(*f),
        Some(other) => Err(format!(
            "{}: argument {} ('{}') must be a number, got {}",
            fn_name,
            idx + 1,
            name,
            other.type_name()
        )),
        None => Err(format!(
            "{}: missing argument {} ('{}')",
            fn_name,
            idx + 1,
            name
        )),
    }
}

/// `device_open(backend, bounds?) -> Device`
pub(crate) fn builtin_device_open(args: &[Value]) -> Result<Value, String> {
    let fn_name = "device_open";
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "{}: expects 1 or 2 arguments (backend, bounds?), got {}",
            fn_name,
            args.len()
        ));
    }
    let backend = expect_string_arg(fn_name, args, 0)?;
    let bounds = match args.get(1) {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(other) => Err(format!(
            "{}: bounds must be String (the verbatim STL-style declaration), got {}",
            fn_name,
            other.type_name()
        ))?,
    };
    embodied::device_open(fn_name, &backend, bounds)
}

/// `bounds_attach(device, formula) -> Device`
pub(crate) fn builtin_bounds_attach(args: &[Value]) -> Result<Value, String> {
    let fn_name = "bounds_attach";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (device, formula), got {}",
            fn_name,
            args.len()
        ));
    }
    let device_id = embodied::device_id_arg(fn_name, args, 0)?;
    let formula = expect_string_arg(fn_name, args, 1)?;
    embodied::bounds_attach(fn_name, &device_id, &formula)
}

/// `device_state(device) -> Struct`
pub(crate) fn builtin_device_state(args: &[Value]) -> Result<Value, String> {
    let fn_name = "device_state";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (device), got {}",
            fn_name,
            args.len()
        ));
    }
    let device_id = embodied::device_id_arg(fn_name, args, 0)?;
    embodied::device_state(fn_name, &device_id)
}

/// `world_state(device) -> WorldState`
pub(crate) fn builtin_world_state(args: &[Value]) -> Result<Value, String> {
    let fn_name = "world_state";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (device), got {}",
            fn_name,
            args.len()
        ));
    }
    let device_id = embodied::device_id_arg(fn_name, args, 0)?;
    embodied::world_state(fn_name, &device_id)
}

/// `pose_make(x, y, z, yaw) -> Pose`
pub(crate) fn builtin_pose_make(args: &[Value]) -> Result<Value, String> {
    let fn_name = "pose_make";
    if args.len() != 4 {
        return Err(format!(
            "{}: expects 4 arguments (x, y, z, yaw), got {}",
            fn_name,
            args.len()
        ));
    }
    let x = expect_float_arg(fn_name, args, 0, "x")?;
    let y = expect_float_arg(fn_name, args, 1, "y")?;
    let z = expect_float_arg(fn_name, args, 2, "z")?;
    let yaw = expect_float_arg(fn_name, args, 3, "yaw")?;
    embodied::pose_make(fn_name, x, y, z, yaw)
}

/// `trajectory_make(poses) -> Trajectory`
pub(crate) fn builtin_trajectory_make(args: &[Value]) -> Result<Value, String> {
    let fn_name = "trajectory_make";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (a List of Pose), got {}",
            fn_name,
            args.len()
        ));
    }
    match &args[0] {
        Value::List(items) => embodied::trajectory_make(fn_name, items),
        other => Err(format!(
            "{}: the trajectory points must be a List of Pose, got {}",
            fn_name,
            other.type_name()
        )),
    }
}

/// `goal_make(predicate) -> GoalPredicate`
pub(crate) fn builtin_goal_make(args: &[Value]) -> Result<Value, String> {
    let fn_name = "goal_make";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (predicate), got {}",
            fn_name,
            args.len()
        ));
    }
    let text = expect_string_arg(fn_name, args, 0)?;
    embodied::goal_make(fn_name, &text)
}

/// `chunk_make(device, trajectory, goal?) -> ActionChunk`
pub(crate) fn builtin_chunk_make(args: &[Value]) -> Result<Value, String> {
    let fn_name = "chunk_make";
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "{}: expects 2 or 3 arguments (device, trajectory, goal?), got {}",
            fn_name,
            args.len()
        ));
    }
    let device_id = embodied::device_id_arg(fn_name, args, 0)?;
    let trajectory_id = embodied::trajectory_id_arg(fn_name, args, 1)?;
    let goal_id = match args.get(2) {
        None => None,
        Some(v) => Some(embodied::goal_id_arg(fn_name, std::slice::from_ref(v), 0)?),
    };
    embodied::chunk_make(fn_name, &device_id, &trajectory_id, goal_id.as_deref())
}

/// `proof_seal(world, chunk, telemetry?) -> Proof`
pub(crate) fn builtin_proof_seal(args: &[Value]) -> Result<Value, String> {
    let fn_name = "proof_seal";
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "{}: expects 2 or 3 arguments (world, chunk, telemetry?), got {}",
            fn_name,
            args.len()
        ));
    }
    let world_id = embodied::world_id_arg(fn_name, args, 0)?;
    let chunk_id = embodied::chunk_id_arg(fn_name, args, 1)?;
    let telemetry = match args.get(2) {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(other) => Err(format!(
            "{}: telemetry must be String, got {}",
            fn_name,
            other.type_name()
        ))?,
    };
    embodied::proof_seal(fn_name, &world_id, &chunk_id, telemetry)
}

/// `proof_verify(proof) -> Struct`
pub(crate) fn builtin_proof_verify(args: &[Value]) -> Result<Value, String> {
    let fn_name = "proof_verify";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (proof), got {}",
            fn_name,
            args.len()
        ));
    }
    let proof_id = embodied::proof_id_arg(fn_name, args, 0)?;
    embodied::proof_verify(fn_name, &proof_id)
}

/// The WorldState materialization guard for `to_string` (the
/// private-by-default leg — typed refusal + the audit record; every
/// other embodied handle projects through its opaque Display marker).
pub(crate) fn guard_world_state_to_string(v: &Value) -> Result<(), String> {
    if let Value::WorldState(_) = v {
        return Err(embodied::deny_world_state_materialization("to_string", v));
    }
    Ok(())
}

/// The same guard for `json_encode` (the map projection would otherwise
/// serialize — the one surface where the projection becomes content).
pub(crate) fn guard_world_state_json(v: &Value) -> Result<(), String> {
    if let Value::WorldState(_) = v {
        return Err(embodied::deny_world_state_materialization("json_encode", v));
    }
    Ok(())
}

/// The `print`-side helper: the typed WorldState leg carries its own
/// audit record; the remaining opaque handles keep the generic refusal.
pub(crate) fn check_print_arg(v: &Value) -> Result<(), String> {
    if let Value::WorldState(_) = v {
        return Err(embodied::deny_world_state_materialization("print", v));
    }
    if crate::interpreter::values::is_nonprintable(v) {
        return Err(format!(
            "print() refused: {} values cannot be printed (Secret and other opaque types)",
            v.type_name()
        ));
    }
    Ok(())
}
