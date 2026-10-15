# Topic 5 — State and memory

**Status: normative (the №645 protocol).** Every norm below was probed
on BOTH backends before it was written; the conformance pairs prove the
record on every CI run. This topic's probe found NO cross-backend
divergence: the world_state chain and the memory round-trip refuse and
answer identically on the TW and the VM (the embodied refusals are
shared-code builtins; the memory persistence is the shared
memory_ops/store layer).

The embodied contour's DESIGN (sim-only, the STL monitor, the GPU-gated
stages) is [ADR-0159](../adr/0159-sim-first-stl.md)'s record — this
topic fixates the language-facing state contracts only.

### S-ST-001 — world_state is private by default; materialization refuses

`world_state(device)` snapshots the deterministic stage-A world state
and returns the OPAQUE WorldState handle. The handle carries the
PRIVATE label (the №355/ADR-0159 posture): every MATERIALIZATION
surface — `print`, `to_string`, `json_encode` — refuses at RUNTIME
with the typed `WORLD_STATE_PRIVATE` code and an audit record, on BOTH
backends. The refusal is the default contract, not a profile opt-in.
**Anchors:** `builtin_world_state` (src/builtins/embodied.rs — the
chain `device_id_arg` → `embodied::world_state`); the materialization
refusals `deny_world_state_materialization` (src/embodied.rs:528+;
wired at src/builtins/embodied.rs:242–260); the stable code
`CODE_WORLD_STATE_PRIVATE` (src/interpreter/values.rs:592).
**Conformance:** `tests/conformance/sst_001_world_state_materialization_refused.mlog`.

### S-ST-002 — The private label gates materialization, not the handle flow

The WorldState handle FLOWS as an opaque value: it passes through
pattern parameters and returns like any first-class value; only the
MATERIALIZATION surfaces are gated (S-ST-001). The device chain is
typed fail-closed: `device_open(backend)` resolves the backend in
`BACKEND_REGISTRY` and requires the `embodied-sim` class — an unknown
backend is `EMBODIED_BACKEND_UNKNOWN`, a non-sim class is
`EMBODIED_CLASS_MISMATCH` (a device over a TTS weights backend is a
category error, not a degraded mode).
**Anchors:** `embodied::device_open` (src/embodied.rs:287–315 — the
registry resolution + the class check); the registry's embodied-sim
records (src/backends.rs:323+, `embodied-sim-kinematic`/
`embodied-mock-device`).
**Conformance:** `tests/conformance/sst_002_world_state_handle_flows_ok.mlog`.

### S-ST-003 — The literal-key memory round-trip

`memorize(key, value)` persists under the literal key;
`recall(key)` reads the SAME value back — the round-trip holds on BOTH
backends (the shared memory_ops/store layer; the run-mode store is the
in-process store — the persistence boundary is the operator's data
dir, never the repo tree). The key-less/dynamic-key cases are the
conservative any-recall fact of the interp contour (№448) — recorded in
[limitations.md](../limitations.md), not re-stated here.
**Anchors:** `memorize_tw`/`memorize_vm` (src/memory_ops.rs:218/502);
the store trait + the in-process default (src/memory_store.rs:70+).
**Conformance:** `tests/conformance/sst_003_memory_literal_roundtrip_ok.mlog`.

## Honest limits (the probe findings this topic records WITHOUT norms)

The probe found NO cross-backend divergence. The by-design boundaries
this topic operates UNDER are mirrored already — cited here, not
duplicated:

- **№355 — the embodied contour is SIM-ONLY** (seven opaque handles
  over in-tree deterministic records; no hardware, no physics, no
  monitor — the STL evaluation and the state evolution are №356 BEHIND
  the GPU-budget gate; a Proof's stage-A verdict is PENDING by
  construction): the full record is the embodied row in
  [limitations.md](../limitations.md) (the ADR-0159 carriers note).
  The norms S-ST-001/002 fixate the language-facing half of it.
- **№668 — the WorldState registry row stays typed-coarse honestly**:
  the handle is neither scalar nor a parameterized List/Struct, so the
  sensitive-surface row does not close on it; closing would need the
  №604-dictionary decision — the separate OWNER lane (the same fact
  the №688 fixed-list goal counts AROUND — the coarse rows are not
  the untyped 67).
- The world_state privacy refusal is RUNTIME (the materialization
  surfaces), while the STATIC contour sees the handles as opaque
  markers — the static WORLD_STATE_EGRESS check is the lattice/monitor
  contour (№356, gated); the honest asymmetry lives in the embodied
  limitations row too.
