// ── Naryad #355 (registry В5 — Phase 5 «Embodied, sim-only», ADR-0159):
//    the embodied type surfaces ─────────────────────────────────────────
//
// Seven opaque handles per the registry §16.7 item №355 and the ADR-0159
// semantics: DeviceHandle / WorldState / ActionChunk / Pose / Trajectory /
// GoalPredicate / Proof. The registry (this module) is the state; every
// `Value` variant carries only the printable projection map (the ADR-0114
// opaque pattern, the Session/Memory/Duplex precedents).
//
// Contract (ADR-0159 + the naryad):
//   - SIM-ONLY, loud boundary: there is no real hardware and no physics —
//     the contour is the registry + the deterministic mock records. The
//     monitor (STL evaluation) and the state evolution land with №356,
//     BEHIND the GPU-budget gate; the TYPES are carrier-independent.
//   - NO UNMONITORED ACTION (ADR-0159 §2.4.2): an ActionChunk whose device
//     profile carries no bounds formula is refused fail-closed
//     (EMBODIED_UNBOUNDED) — the static contour refuses the flow.
//   - WorldState is PRIVATE-BY-DEFAULT (the Phase-1 lattice: label is
//     carried in the projection; materialization — print / to_string /
//     json_encode — is refused fail-closed with the typed
//     WORLD_STATE_PRIVATE stamp and an audit record, consistent with the
//     №349 duty rule's materialization refusals).
//   - Bounds formulas are private-by-default state too (ADR-0159 §2.4.4):
//     the verbatim text never enters a ledger detail, a Display, or a
//     projection — digests only.
//   - Proof is the SIGNED trace {hash(world), chunk, bounds, telemetry,
//     verdict, hash(sim)} (the №343 contract): sealed records carry a
//     sha256 signature over the canonical field serialization; verify
//     recomputes it and checks the CONTRACT FIELDS ONLY (verdict word,
//     bounds resolution, backend pin/class) — execution is №356.
//   - EVERY verdict/state transition is an Action-Ledger record (the
//     `embodied.*` family, the duplex.*/memory.* template, ADR-0159
//     §2.4.3).

use crate::backends::{BackendClass, ShaPin, BACKEND_REGISTRY};
use crate::interpreter::values::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// The signing context of the stage-A Proof seal (the №343 signed-trace
/// contract). The in-tree constant binds the signature to the embodied
/// surface; the monitor-backed sealing (№356) reuses the same context.
pub const PROOF_SIGNING_CONTEXT: &str =
    "metalogos-embodied-proof-v1 (naryad 355 / 343; stage-A mock contour)";

/// The trajectory capacity guard — a loud refusal, not a silent truncation.
pub const MAX_TRAJECTORY_POINTS: usize = 1024;

/// The typed STL verdict (ADR-0159 §2.2): `Satisfied(t)` /
/// `Violated(t, signal, observed, bound)` / `Pending`. The verdict is
/// data — it goes into the Proof trace and the ledger. №355 records the
/// words; the evaluation lands with the №356 monitor.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Satisfied,
    Violated {
        signal: String,
        observed: String,
        bound: String,
    },
    Pending,
}

impl Verdict {
    /// The closed verdict vocabulary word.
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Satisfied => "satisfied",
            Verdict::Violated { .. } => "violated",
            Verdict::Pending => "pending",
        }
    }

    /// Parse the closed vocabulary. Unknown words are loud (never a
    /// silent Pending — a forged or garbled verdict must not masquerade
    /// as a well-formed one).
    pub fn parse(word: &str) -> Option<Verdict> {
        match word {
            "satisfied" => Some(Verdict::Satisfied),
            "violated" => Some(Verdict::Violated {
                signal: String::new(),
                observed: String::new(),
                bound: String::new(),
            }),
            "pending" => Some(Verdict::Pending),
            _ => None,
        }
    }

    /// The violation detail (empty for the other verdicts).
    pub fn violation_detail(&self) -> Option<(&str, &str, &str)> {
        match self {
            Verdict::Violated {
                signal,
                observed,
                bound,
            } => Some((signal.as_str(), observed.as_str(), bound.as_str())),
            _ => None,
        }
    }
}

/// The bounds formula — the verbatim STL-style text, stored OPAQUE. The
/// monitor (№356) parses it; nothing in this module ever prints it
/// (ADR-0159 §2.4.4: private-by-default state).
#[derive(Debug, Clone)]
pub struct BoundsFormula {
    pub id: String,
    pub text: String,
}

impl BoundsFormula {
    /// The sha256 digest of the verbatim text (the only form the text
    /// may take outside the verified contour).
    pub fn digest(&self) -> String {
        crate::ledger::sha256_hex(self.text.as_bytes())
    }
}

#[derive(Debug, Clone)]
pub struct DeviceProfile {
    pub id: String,
    /// The program-visible backend name — MUST resolve in
    /// BACKEND_REGISTRY with class `embodied-sim` (the В2 profile: the
    /// registry is the SSOT for what a backend IS, ADR-0163).
    pub backend: String,
    pub bounds: Option<BoundsFormula>,
}

/// The world snapshot — PRIVATE by default (the Phase-1 lattice label).
/// The deterministic stage-A snapshot: step 0, digest over the device
/// profile. The state evolution lands with №356.
#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    pub id: String,
    pub device: String,
    pub step: u64,
    /// sha256 over the canonical profile — the hash(world) leg of the
    /// Proof contract.
    pub digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
}

#[derive(Debug, Clone)]
pub struct TrajectoryData {
    pub points: Vec<PoseData>,
}

/// The goal predicate — the verbatim text, opaque (evaluated by №356).
#[derive(Debug, Clone)]
pub struct GoalData {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ChunkData {
    pub id: String,
    pub device: String,
    pub trajectory: String,
    pub goal: Option<String>,
    /// The bounds formula the device profile carried at make time —
    /// the chunk is UNMAKABLE without one (ADR-0159 §2.4.2).
    pub bounds: String,
}

/// The signed Proof record (the №343 contract fields):
/// {hash(world), chunk, bounds, telemetry, verdict, hash(sim)}.
#[derive(Debug, Clone)]
pub struct ProofRecord {
    pub id: String,
    pub world_hash: String,
    pub chunk_id: String,
    pub bounds_id: String,
    /// The telemetry DIGEST (sha256) — verbatim telemetry never enters
    /// the record (the №415 posture: digests, not payloads).
    pub telemetry_digest: String,
    pub verdict: Verdict,
    pub sim_hash: String,
    /// sha256 over the canonical field serialization + the signing
    /// context — the integrity of the recorded trace.
    pub signature: String,
}

impl ProofRecord {
    /// The canonical serialization the signature covers.
    fn canonical(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            PROOF_SIGNING_CONTEXT,
            self.world_hash,
            self.chunk_id,
            self.bounds_id,
            self.telemetry_digest,
            self.verdict.as_str(),
            self.sim_hash,
            self.violation_canonical(),
        )
    }

    fn violation_canonical(&self) -> String {
        match &self.verdict {
            Verdict::Violated {
                signal,
                observed,
                bound,
            } => format!("violated|{}|{}|{}", signal, observed, bound),
            other => other.as_str().to_string(),
        }
    }

    pub fn sign(&mut self) {
        self.signature = crate::ledger::sha256_hex(self.canonical().as_bytes());
    }

    /// The signature check — recomputed over the CURRENT fields.
    pub fn signature_valid(&self) -> bool {
        self.signature == crate::ledger::sha256_hex(self.canonical().as_bytes())
    }
}

/// The registry state — one mutex, seven maps (the duplex.rs template).
#[derive(Default)]
pub struct EmbodiedState {
    pub devices: HashMap<String, DeviceProfile>,
    pub worlds: HashMap<String, WorldSnapshot>,
    pub poses: HashMap<String, PoseData>,
    pub trajectories: HashMap<String, TrajectoryData>,
    pub goals: HashMap<String, GoalData>,
    pub chunks: HashMap<String, ChunkData>,
    pub proofs: HashMap<String, ProofRecord>,
}

fn state() -> &'static Mutex<EmbodiedState> {
    static REG: OnceLock<Mutex<EmbodiedState>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(EmbodiedState::default()))
}

static SEQ: AtomicU64 = AtomicU64::new(1);

fn fresh_id(prefix: &str, seed: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let preimage = format!("{}|{}|{}|{}", seed, millis, seq, prefix);
    format!(
        "{}-{}",
        prefix,
        &crate::ledger::sha256_hex(preimage.as_bytes())[..16]
    )
}

/// The Action-Ledger record (`embodied.*` family — the duplex.*
/// template, best-effort per ADR-0167 §2 driver 5).
fn ledger_embodied_event(kind: &str, id: &str, detail: &str) {
    eprintln!("[EMBODIED_{}] {} {}", kind.to_uppercase(), id, detail);
    crate::ledger::record(&format!("embodied.{}", kind), id, "embodied", detail);
}

// ── The typed origin stamps (the №413 convention — position-0 markers
//    the `try` classifier branches on; whitelisted in values.rs). ──────

fn err_handle_unknown(fn_name: &str, kind: &str, id: &str) -> String {
    crate::interpreter::values::coded_error(
        crate::interpreter::values::CODE_EMBODIED_HANDLE_UNKNOWN,
        format!(
            "{}: unknown {} handle '{}' — open one with the embodied constructors first",
            fn_name, kind, id
        ),
    )
}

// ── Device handles ──────────────────────────────────────────────────────

/// `device_open(backend, bounds?)` — open a SIM device handle. The
/// backend MUST resolve in BACKEND_REGISTRY with the `embodied-sim`
/// class (fail-closed typed refusals otherwise — a device over a TTS
/// weights backend is a category error, not a degraded mode).
pub fn device_open(
    fn_name: &str,
    backend: &str,
    bounds_text: Option<&str>,
) -> Result<Value, String> {
    let entry = crate::backends::find_by_name(backend).ok_or_else(|| {
        crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_EMBODIED_BACKEND_UNKNOWN,
            format!(
                "{}: unknown backend '{}' — the embodied contour runs on the registry's embodied-sim records ({})",
                fn_name,
                backend,
                embodied_backend_names().join("/")
            ),
        )
    })?;
    if entry.class != BackendClass::EmbodiedSim {
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_EMBODIED_CLASS_MISMATCH,
            format!(
                "{}: backend '{}' is class '{}', not 'embodied-sim' — devices open on sim records only (the contour is sim-only, ADR-0159)",
                fn_name, backend, entry.class.as_str()
            ),
        ));
    }
    let bounds = match bounds_text {
        Some(text) => {
            let text = text.trim();
            if text.is_empty() {
                return Err(format!(
                    "{}: the bounds formula must be a non-empty declaration (the verbatim STL-style text; the monitor parses it in №356)",
                    fn_name
                ));
            }
            Some(BoundsFormula {
                id: fresh_id("bnd", backend),
                text: text.to_string(),
            })
        }
        None => None,
    };
    let mut reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let device = DeviceProfile {
        id: fresh_id("dev", backend),
        backend: backend.to_string(),
        bounds,
    };
    let bounds_word = if device.bounds.is_some() {
        "attached"
    } else {
        "none"
    };
    ledger_embodied_event(
        "device_open",
        &device.id,
        &format!("backend={}|bounds={}", backend, bounds_word),
    );
    let value = device_value(&device);
    reg.devices.insert(device.id.clone(), device);
    Ok(value)
}

/// `bounds_attach(device, formula)` — attach (or replace) the bounds
/// formula on a device profile. The verbatim text is recorded as a
/// DIGEST only (private-by-default state, ADR-0159 §2.4.4).
pub fn bounds_attach(fn_name: &str, device_id: &str, text: &str) -> Result<Value, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(format!(
            "{}: the bounds formula must be a non-empty declaration",
            fn_name
        ));
    }
    let mut reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let device = reg
        .devices
        .get_mut(device_id)
        .ok_or_else(|| err_handle_unknown(fn_name, "device", device_id))?;
    let formula = BoundsFormula {
        id: fresh_id("bnd", &device.backend),
        text: text.to_string(),
    };
    ledger_embodied_event(
        "bounds_attach",
        &device.id,
        &format!("bounds_digest={}", formula.digest()),
    );
    device.bounds = Some(formula);
    let value = device_value(device);
    Ok(value)
}

/// `device_state(device)` — the introspection projection (metadata
/// only; the bounds text itself never surfaces — presence and digest).
pub fn device_state(fn_name: &str, device_id: &str) -> Result<Value, String> {
    let reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let device = reg
        .devices
        .get(device_id)
        .ok_or_else(|| err_handle_unknown(fn_name, "device", device_id))?;
    let fields: Vec<(&str, Value)> = vec![
        ("id", Value::String(device.id.clone())),
        ("backend", Value::String(device.backend.clone())),
        ("bounds_present", Value::Bool(device.bounds.is_some())),
        (
            "bounds_digest",
            match &device.bounds {
                Some(f) => Value::String(f.digest()),
                None => Value::Unit,
            },
        ),
    ];
    ledger_embodied_event("device_state", &device.id, "introspection");
    Ok(make_struct("EmbodiedDeviceState", fields))
}

fn device_value(d: &DeviceProfile) -> Value {
    Value::Device(HashMap::from([
        ("id".to_string(), d.id.clone()),
        ("backend".to_string(), d.backend.clone()),
    ]))
}

/// The device id from a `Value::Device` handle argument.
pub fn device_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::Device(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: Device handle has no id field — open devices with device_open",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected Device as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!("{}: missing Device argument {}", fn_name, idx + 1)),
    }
}

/// The names of the registry's embodied-sim records (the error text).
pub fn embodied_backend_names() -> Vec<&'static str> {
    BACKEND_REGISTRY
        .iter()
        .filter(|e| e.class == BackendClass::EmbodiedSim)
        .map(|e| e.name)
        .collect()
}

// ── WorldState (private-by-default) ─────────────────────────────────────

/// `world_state(device)` — the deterministic stage-A snapshot (step 0).
/// PRIVATE by default: the projection carries `label: "private"` and
/// every materialization surface refuses it (the WorldState-private
/// contract; the №349 duty-rule consistency).
pub fn world_state(fn_name: &str, device_id: &str) -> Result<Value, String> {
    let mut reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let device = reg
        .devices
        .get(device_id)
        .ok_or_else(|| err_handle_unknown(fn_name, "device", device_id))?
        .clone();
    let canonical = format!(
        "world|{}|{}|{}",
        device.backend,
        device
            .bounds
            .as_ref()
            .map(|b| b.digest())
            .unwrap_or_else(|| "unbounded".to_string()),
        0u64,
    );
    let snapshot = WorldSnapshot {
        id: fresh_id("wld", &device.id),
        device: device.id.clone(),
        step: 0,
        digest: crate::ledger::sha256_hex(canonical.as_bytes()),
    };
    ledger_embodied_event(
        "world_state",
        &snapshot.id,
        &format!("device={}|step=0|digest={}", device.id, snapshot.digest),
    );
    let value = world_value(&snapshot);
    reg.worlds.insert(snapshot.id.clone(), snapshot);
    Ok(value)
}

fn world_value(w: &WorldSnapshot) -> Value {
    Value::WorldState(HashMap::from([
        ("id".to_string(), w.id.clone()),
        ("device".to_string(), w.device.clone()),
        // The loud Phase-1 label — private by default (№349/№355 lattice).
        ("label".to_string(), "private".to_string()),
        ("step".to_string(), w.step.to_string()),
    ]))
}

/// The world id from a `Value::WorldState` handle argument.
pub fn world_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::WorldState(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: WorldState handle has no id field — snapshot one with world_state",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected WorldState as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!(
            "{}: missing WorldState argument {}",
            fn_name,
            idx + 1
        )),
    }
}

/// The WorldState materialization refusal — the shared engine of the
/// print / to_string / json_encode guards. Fail-closed: the typed
/// WORLD_STATE_PRIVATE stamp (try-branchable, the №413 convention) and
/// an audit record — no silent egress AND no silent refusal (the №428
/// posture).
pub fn deny_world_state_materialization(surface: &str, v: &Value) -> String {
    let id = match v {
        Value::WorldState(map) => map
            .get("id")
            .cloned()
            .unwrap_or_else(|| "<unbound>".to_string()),
        _ => "<unbound>".to_string(),
    };
    ledger_embodied_event(
        "world_state_denied",
        &id,
        &format!("surface={}|reason=private-materialization", surface),
    );
    crate::interpreter::values::coded_error(
        crate::interpreter::values::CODE_WORLD_STATE_PRIVATE,
        format!(
            "{} refused: WorldState is private-by-default state — its materialization outside the verified contour is an error (the Phase-1 lattice / №349 consistency; inspect via the introspection surfaces)",
            surface
        ),
    )
}

/// Introspection for tests: the world snapshot's label word (always
/// "private" today — the lattice materialization is the №356 contour).
pub fn world_label(world_id: &str) -> Option<String> {
    let reg = state().lock().ok()?;
    reg.worlds.get(world_id).map(|_| "private".to_string())
}

// ── Pose / Trajectory / GoalPredicate ───────────────────────────────────

/// `pose_make(x, y, z, yaw)` — the pose handle. The numeric payload
/// lives in the registry (the opaque pattern: bulky payloads never
/// enter `Value` — the ADR-0114 Reflex rationale).
pub fn pose_make(fn_name: &str, x: f64, y: f64, z: f64, yaw: f64) -> Result<Value, String> {
    for (name, v) in [("x", x), ("y", y), ("z", z), ("yaw", yaw)] {
        if !v.is_finite() {
            return Err(format!(
                "{}: coordinate '{}' must be a finite number, got {}",
                fn_name, name, v
            ));
        }
    }
    let mut reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let id = fresh_id("pos", &format!("{}|{}|{}|{}", x, y, z, yaw));
    reg.poses.insert(id.clone(), PoseData { x, y, z, yaw });
    Ok(Value::Pose(HashMap::from([
        ("id".to_string(), id),
        ("frame".to_string(), "world".to_string()),
    ])))
}

/// `trajectory_make(list)` — the trajectory handle over poses. Empty
/// and over-capacity trajectories refuse loudly (no silent truncation).
pub fn trajectory_make(fn_name: &str, poses: &[Value]) -> Result<Value, String> {
    if poses.is_empty() {
        return Err(format!(
            "{}: an empty trajectory is refused — at least one Pose is required (no silent no-op)",
            fn_name
        ));
    }
    if poses.len() > MAX_TRAJECTORY_POINTS {
        return Err(format!(
            "{}: trajectory exceeds the capacity guard ({} > {} points) — refused loudly, not truncated",
            fn_name,
            poses.len(),
            MAX_TRAJECTORY_POINTS
        ));
    }
    let mut reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let mut points = Vec::with_capacity(poses.len());
    for (i, p) in poses.iter().enumerate() {
        match p {
            Value::Pose(map) => {
                let pid = map.get("id").cloned().unwrap_or_default();
                let data = reg
                    .poses
                    .get(&pid)
                    .ok_or_else(|| err_handle_unknown(fn_name, "pose", &pid))?;
                points.push(*data);
            }
            other => {
                return Err(format!(
                    "{}: trajectory point {} must be Pose, got {}",
                    fn_name,
                    i + 1,
                    other.type_name()
                ))
            }
        }
    }
    let id = fresh_id("trj", &format!("points={}", points.len()));
    let count = points.len();
    reg.trajectories
        .insert(id.clone(), TrajectoryData { points });
    Ok(Value::Trajectory(HashMap::from([
        ("id".to_string(), id),
        ("points".to_string(), count.to_string()),
    ])))
}

/// The trajectory id from a `Value::Trajectory` handle argument.
pub fn trajectory_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::Trajectory(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: Trajectory handle has no id field — build one with trajectory_make",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected Trajectory as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!(
            "{}: missing Trajectory argument {}",
            fn_name,
            idx + 1
        )),
    }
}

/// `goal_make(predicate)` — the goal-predicate handle (the verbatim
/// text, opaque; evaluated by the №356 monitor).
pub fn goal_make(fn_name: &str, text: &str) -> Result<Value, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(format!(
            "{}: the goal predicate must be a non-empty declaration",
            fn_name
        ));
    }
    let mut reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let goal = GoalData {
        id: fresh_id("gol", text),
        text: text.to_string(),
    };
    let id = goal.id.clone();
    reg.goals.insert(id.clone(), goal);
    Ok(Value::GoalPredicate(HashMap::from([(
        "id".to_string(),
        id,
    )])))
}

/// The goal id from a `Value::GoalPredicate` handle argument.
pub fn goal_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::GoalPredicate(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: GoalPredicate handle has no id field — declare one with goal_make",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected GoalPredicate as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!(
            "{}: missing GoalPredicate argument {}",
            fn_name,
            idx + 1
        )),
    }
}

// ── ActionChunk (no unmonitored action — ADR-0159 §2.4.2) ───────────────

/// `chunk_make(device, trajectory, goal?)` — the action chunk. A device
/// profile without a bounds formula refuses (EMBODIED_UNBOUNDED,
/// fail-closed): the static contour refuses the flow before it runs.
pub fn chunk_make(
    fn_name: &str,
    device_id: &str,
    trajectory_id: &str,
    goal_id: Option<&str>,
) -> Result<Value, String> {
    let mut reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let device = reg
        .devices
        .get(device_id)
        .ok_or_else(|| err_handle_unknown(fn_name, "device", device_id))?
        .clone();
    let bounds_id = match &device.bounds {
        Some(f) => f.id.clone(),
        None => {
            ledger_embodied_event(
                "chunk_denied",
                device_id,
                "reason=unbounded (ADR-0159 §2.4.2: no unmonitored action)",
            );
            return Err(crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_EMBODIED_UNBOUNDED,
                format!(
                    "{}: device '{}' carries no bounds formula — an ActionChunk without a bounds formula bound to its device profile is refused at the type level (ADR-0159 §2.4.2); attach one with bounds_attach",
                    fn_name, device_id
                ),
            ));
        }
    };
    if !reg.trajectories.contains_key(trajectory_id) {
        return Err(err_handle_unknown(fn_name, "trajectory", trajectory_id));
    }
    if let Some(g) = goal_id {
        if !reg.goals.contains_key(g) {
            return Err(err_handle_unknown(fn_name, "goal", g));
        }
    }
    let chunk = ChunkData {
        id: fresh_id("chk", &format!("{}|{}", device_id, trajectory_id)),
        device: device_id.to_string(),
        trajectory: trajectory_id.to_string(),
        goal: goal_id.map(|g| g.to_string()),
        bounds: bounds_id.clone(),
    };
    ledger_embodied_event(
        "chunk_make",
        &chunk.id,
        &format!(
            "device={}|bounds={}|goal={}",
            device_id,
            &bounds_id[..12.min(bounds_id.len())],
            goal_id.unwrap_or("none")
        ),
    );
    let value = chunk_value(&chunk);
    reg.chunks.insert(chunk.id.clone(), chunk);
    Ok(value)
}

fn chunk_value(c: &ChunkData) -> Value {
    Value::ActionChunk(HashMap::from([
        ("id".to_string(), c.id.clone()),
        ("device".to_string(), c.device.clone()),
        ("trajectory".to_string(), c.trajectory.clone()),
        ("goal".to_string(), c.goal.clone().unwrap_or_default()),
    ]))
}

/// The chunk id from a `Value::ActionChunk` handle argument.
pub fn chunk_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::ActionChunk(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: ActionChunk handle has no id field — build one with chunk_make",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected ActionChunk as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!(
            "{}: missing ActionChunk argument {}",
            fn_name,
            idx + 1
        )),
    }
}

// ── Proof (the signed trace — the №343 contract) ────────────────────────

/// `proof_seal(world, chunk, telemetry?)` — seal the signed trace. The
/// stage-A verdict is Pending by construction: the monitor lands with
/// №356, and a Pending proof is the only honest seal (no program-forged
/// "satisfied"). Telemetry is recorded as a DIGEST only.
pub fn proof_seal(
    fn_name: &str,
    world_id: &str,
    chunk_id: &str,
    telemetry: Option<&str>,
) -> Result<Value, String> {
    let mut reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let world = reg
        .worlds
        .get(world_id)
        .ok_or_else(|| err_handle_unknown(fn_name, "world", world_id))?
        .clone();
    let chunk = reg
        .chunks
        .get(chunk_id)
        .ok_or_else(|| err_handle_unknown(fn_name, "chunk", chunk_id))?
        .clone();
    if chunk.device != world.device {
        return Err(format!(
            "{}: chunk '{}' is bound to device '{}', but the world '{}' belongs to device '{}' — a proof seals a chunk over ITS OWN world (fail-closed)",
            fn_name, chunk_id, chunk.device, world_id, world.device
        ));
    }
    let telemetry_digest = match telemetry {
        Some(t) if !t.trim().is_empty() => crate::ledger::sha256_hex(t.trim().as_bytes()),
        _ => crate::ledger::sha256_hex(b"no-telemetry"),
    };
    let sim_hash = crate::ledger::sha256_hex(
        format!(
            "sim|{}|{}|{}|{}",
            world.digest, chunk.id, chunk.bounds, chunk.trajectory
        )
        .as_bytes(),
    );
    let mut proof = ProofRecord {
        id: fresh_id("prf", &format!("{}|{}", world_id, chunk_id)),
        world_hash: world.digest.clone(),
        chunk_id: chunk.id.clone(),
        bounds_id: chunk.bounds.clone(),
        telemetry_digest,
        verdict: Verdict::Pending,
        sim_hash,
        signature: String::new(),
    };
    proof.sign();
    ledger_embodied_event(
        "proof_seal",
        &proof.id,
        &format!(
            "world={}|chunk={}|bounds={}|verdict=pending",
            world_id, chunk_id, chunk.bounds
        ),
    );
    let value = proof_value(&proof);
    reg.proofs.insert(proof.id.clone(), proof);
    Ok(value)
}

fn proof_value(p: &ProofRecord) -> Value {
    Value::Proof(HashMap::from([
        ("id".to_string(), p.id.clone()),
        ("verdict".to_string(), p.verdict.as_str().to_string()),
    ]))
}

/// The proof id from a `Value::Proof` handle argument.
pub fn proof_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::Proof(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: Proof handle has no id field — seal one with proof_seal",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected Proof as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!("{}: missing Proof argument {}", fn_name, idx + 1)),
    }
}

/// The contract-field validation of a resolved record (the data engine
/// behind `proof_verify`): signature integrity, verdict vocabulary,
/// bounds resolution, backend pin/class — EXECUTION is №356. Takes the
/// ALREADY-LOCKED registry state (the std Mutex is not reentrant —
/// proof_verify runs under its guard).
pub fn validate_proof_record(record: &ProofRecord, reg: &EmbodiedState) -> Vec<String> {
    let mut reasons = Vec::new();
    if !record.signature_valid() {
        reasons.push("signature does not match the canonical field serialization (tampered or foreign trace)".to_string());
    }
    if Verdict::parse(record.verdict.as_str()).is_none() {
        reasons.push(format!(
            "verdict '{}' is outside the closed vocabulary (satisfied/violated/pending)",
            record.verdict.as_str()
        ));
    }
    if let Some((signal, observed, bound)) = record.verdict.violation_detail() {
        if record.verdict.as_str() == "violated"
            && signal.is_empty()
            && observed.is_empty()
            && bound.is_empty()
        {
            reasons.push("violated verdict carries no signal/observed/bound detail".to_string());
        }
    }
    if record.world_hash.len() != 64 || !record.world_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        reasons.push("world_hash is not a sha256 hex digest".to_string());
    }
    let bounds_known = reg
        .devices
        .values()
        .any(|d| d.bounds.as_ref().map(|b| b.id.as_str()) == Some(record.bounds_id.as_str()));
    if !bounds_known {
        reasons.push(format!(
            "bounds '{}' does not resolve to any device-profile formula",
            record.bounds_id
        ));
    }
    if let Some(chunk) = reg.chunks.get(&record.chunk_id) {
        if let Some(device) = reg.devices.get(&chunk.device) {
            match crate::backends::find_by_name(&device.backend) {
                None => reasons.push(format!(
                    "device backend '{}' is not in BACKEND_REGISTRY",
                    device.backend
                )),
                Some(entry) => {
                    if entry.class != BackendClass::EmbodiedSim {
                        reasons.push(format!(
                            "device backend '{}' is class '{}', not 'embodied-sim'",
                            device.backend,
                            entry.class.as_str()
                        ));
                    }
                    // The pin leg: the entry must DECLARE a pin status
                    // (the №334 contract field — Pinned(h) or the honest
                    // PendingNo334 for in-tree sim records; the DECLARATION
                    // is the contract, the load-time refusal is №334's).
                    let _declared_pin: ShaPin = entry.pin;
                }
            }
        } else {
            reasons.push("the chunk's device no longer resolves".to_string());
        }
    } else {
        reasons.push("the proof's chunk does not resolve".to_string());
    }
    reasons
}

/// `proof_verify(proof)` — the runtime validation of the contract
/// fields (data-level; the execution/re-simulation is №356). Audited.
pub fn proof_verify(fn_name: &str, proof_id: &str) -> Result<Value, String> {
    let reg = state()
        .lock()
        .map_err(|e| format!("embodied registry lock: {}", e))?;
    let record = reg
        .proofs
        .get(proof_id)
        .ok_or_else(|| err_handle_unknown(fn_name, "proof", proof_id))?
        .clone();
    let reasons = validate_proof_record(&record, &reg);
    let valid = reasons.is_empty();
    let backend = reg
        .chunks
        .get(&record.chunk_id)
        .and_then(|c| reg.devices.get(&c.device))
        .map(|d| d.backend.clone())
        .unwrap_or_default();
    ledger_embodied_event(
        "proof_verify",
        &record.id,
        &format!("valid={}|reasons={}", valid, reasons.len()),
    );
    let fields: Vec<(&str, Value)> = vec![
        ("valid", Value::Bool(valid)),
        ("proof_id", Value::String(record.id.clone())),
        (
            "verdict",
            Value::String(record.verdict.as_str().to_string()),
        ),
        ("world_hash", Value::String(record.world_hash.clone())),
        ("chunk_id", Value::String(record.chunk_id.clone())),
        ("bounds_id", Value::String(record.bounds_id.clone())),
        ("backend", Value::String(backend)),
        (
            "reasons",
            Value::List(reasons.iter().map(|r| Value::String(r.clone())).collect()),
        ),
    ];
    Ok(make_struct("EmbodiedProofReport", fields))
}

/// Introspection for tests: a clone of the resolved Proof record.
pub fn proof_record(proof_id: &str) -> Option<ProofRecord> {
    let reg = state().lock().ok()?;
    reg.proofs.get(proof_id).cloned()
}

/// Test seam: insert a record directly (the tamper-detection corpus).
#[doc(hidden)]
pub fn insert_proof_record_for_tests(record: ProofRecord) {
    if let Ok(mut reg) = state().lock() {
        reg.proofs.insert(record.id.clone(), record);
    }
}

/// Test seam: the locked registry state (the tamper corpus validates at
/// the data level against the LIVE state — bounds/chunk resolution —
/// while mutating the record clone freely).
#[doc(hidden)]
pub fn state_for_tests() -> std::sync::MutexGuard<'static, EmbodiedState> {
    state().lock().unwrap_or_else(|e| e.into_inner())
}

// ── The shared struct constructor (the duplex.rs `struct_of` shape) ────

fn make_struct(type_name: &str, fields: Vec<(&str, Value)>) -> Value {
    let map: HashMap<String, Value> = fields
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    Value::Struct {
        type_name: type_name.to_string(),
        fields: map,
    }
}

// ── The registry-profile contract tests (in-module — the В2 profile) ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embodied_sim_records_are_declared() {
        let names = embodied_backend_names();
        assert!(
            names.contains(&"embodied-sim-kinematic"),
            "the kinematic sim record must be in the registry"
        );
        assert!(
            names.contains(&"embodied-mock-device"),
            "the mock device record must be in the registry"
        );
    }

    #[test]
    fn embodied_records_carry_license_classes() {
        // The В2 profile: every record carries a license class + note.
        for e in BACKEND_REGISTRY.iter() {
            if e.class == BackendClass::EmbodiedSim {
                assert!(!e.license_note.is_empty(), "{}: license note", e.name);
                assert!(
                    matches!(e.license, crate::backends::LicenseClass::Osi),
                    "{}: in-tree sim records are governed by the repo license (osi)",
                    e.name
                );
                // The pin contract: the DECLARATION exists (PendingNo334
                // honestly — no external artifact to pin).
                assert!(matches!(e.pin, ShaPin::PendingNo334));
            }
        }
    }

    #[test]
    fn verdict_vocabulary_is_closed() {
        assert_eq!(Verdict::parse("satisfied"), Some(Verdict::Satisfied));
        assert!(Verdict::parse("pending").is_some());
        assert!(Verdict::parse("violated").is_some());
        assert_eq!(Verdict::parse("Satisfied"), None, "case-sensitive");
        assert_eq!(Verdict::parse("ok"), None);
    }

    #[test]
    fn proof_signature_detects_tampering() {
        let mut record = ProofRecord {
            id: "prf-test".to_string(),
            world_hash: "a".repeat(64),
            chunk_id: "chk-test".to_string(),
            bounds_id: "bnd-test".to_string(),
            telemetry_digest: crate::ledger::sha256_hex(b"t"),
            verdict: Verdict::Pending,
            sim_hash: "b".repeat(64),
            signature: String::new(),
        };
        record.sign();
        assert!(record.signature_valid(), "fresh seal is valid");
        record.sim_hash = "c".repeat(64);
        assert!(
            !record.signature_valid(),
            "any field mutation breaks the signature"
        );
    }
}
