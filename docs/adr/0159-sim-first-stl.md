# ADR-0159: Sim-first verification — SafetyBounds as an STL-formula monitor

**Status:** Accepted (filled 2026-09-22 by naryad №354, wave 10 — dispatch #620; the slot was booked 2026-09-14 by №319, issue #406)
**Date:** 2026-09-22
**Naryad:** #354 (issue #617; registry Phase 5 «Embodied, sim-only», §16.7 — items В5 №354–№361)
**Pillar:** embodied (docs/adr only in this naryad — the type surfaces are №355)
**Consumers:** №355 (Embodied-handles — DeviceHandle/WorldState/ActionChunk/Pose/Trajectory/GoalPredicate/Proof carry the bounds), №356–№358 (SimProve stage A / G-ActPhys stage B / injection-proof stage C — BEHIND the GPU-budget gate, owner decision), №340 (grant revocation — the emergency-stop leg), №342 (DenyEvent(SimFailed) — the deny leg)
**Prior art (plan §17):** runtime verification / signal-temporal logic (STL) monitors, control-barrier-function (CBF) shielding, Simplex architecture

## 1. Context

Phase 5 adds an embodied contour: the language can hold device handles and
act over a simulated world. Acting bodies are safety-critical by definition
— a generated action stream must be verifiable against bounds BEFORE and
WHILE it executes. Two facts shape the decision:

1. **There is no real hardware** — the contour is sim-only (loud boundary).
   The №294 hardware gate (real weights) is PARKED; №356–№361 wait for the
   owner's GPU-budget decision. Whatever is built now must not be thrown
   away when hardware arrives.
2. **A bound checked only in sim proves nothing about the real system** —
   and a bound checked only at runtime on the real system has no test
   oracle. The classic escape is a MONITOR: one formal specification,
   evaluated over any signal trace, whatever produced it.

The repo already has every primitive the monitor needs: typed opaque
handles (ADR-0114), the label lattice with consent (ADR-0154), Action
Ledger records (ADR-0167/№393/№415), the grant algebra with cascading
revocation (ADR-0155/№340), and the DenyEvent vocabulary (№392 — the
`SimFailed` reason is the Phase-5 extension №342).

## 2. Decision

### 2.1 Sim-first, one formula, two carriers

**SafetyBounds is an STL-formula monitor: the SAME formula is evaluated
over (a) the simulator's trace in stage A and (b) the real telemetry in
stage C.** Sim-first means: the formula and its monitor exist and are
tested in sim BEFORE any hardware path exists, and the hardware path (if
ever built) reuses the formula verbatim — the carriers differ, the
specification does not. This is what makes the sim investment survive the
GPU/hardware transition: what is proven in sim is the MONITOR and the
formula semantics, not the physics.

### 2.2 SafetyBounds is a formula, not a table

A bounds declaration is a set of STL-style constraints over the action/
state signal vocabulary — e.g. `always(|pose.velocity| <= v_max)`,
`always(distance_to(obstacle) >= d_safe)`, `eventually(within(5 s, at(goal)))`.
The monitor evaluates the formula incrementally over a trace and yields a
typed verdict: `Satisfied(t)` / `Violated(t, signal, observed, bound)` /
`Pending` (the horizon has not closed). The verdict is data: it goes into
the Proof trace (№343/№355), the Action Ledger, and — on violation — into
the stop/deny path (§2.4).

No language surface is fixed in this ADR (that is №355's contract: the
handles and their registry profiles). This ADR fixes the SEMANTICS: bounds
are declarative, monitor-evaluable, carrier-independent.

### 2.3 Formal stage-transition criteria (§9.3 A/B/C)

- **Stage A → Stage B** (sim → physics-in-the-loop): (1) the bounds
  monitor runs the full formula set over every stage-A sim trace with ZERO
  unexplained violations (every violation is either a formula fix or a
  traced sim defect); (2) the injection set (§2.5) passes — the monitor
  catches every injected violation class; (3) the Proof trace round-trips
  (hash(world), chunk, bounds verdict) into the ledger.
- **Stage B → Stage C** (physics sim → real telemetry, BEHIND the hardware
  gate): (1) the SAME formula set evaluates over stage-B traces with the
  monitor's verdicts matching the physical oracle; (2) telemetry-shape
  compatibility is proven — the real telemetry maps onto the same signal
  vocabulary the formula consumes (units, frames, rates fixed in the
  device profile, №360); (3) the ε-divergence invariant (§2.4) is
  measurable end to end.
- **Stage C operation**: the monitor runs ON-LINE over real telemetry;
  a violation is a stop + deny, never a warning.

### 2.4 Invariants — divergence ε → stop + deny

1. **ε-divergence**: if the real (or stage-B) trace diverges from the sim
   prediction by more than the declared ε on any monitored signal, the
   monitor's verdict is `Violated` — the action chunk is stopped and the
   deny path fires: `DenyEvent(SimFailed)` (the №342 vocabulary extension)
   and the actor's grants are revoked for the session (the №340 cascading
   revocation — the emergency-stop leg). Nothing about the divergence is
   advisory.
2. **No unmonitored action**: an ActionChunk without a bounds formula
   bound to its device profile is refused at the type level (№355) — the
   static contour refuses the flow before it runs.
3. **Audit completeness**: every monitor verdict (satisfied, violated,
   pending-closed) is an Action-Ledger record (`embodied.*` family — the
   naming follows the `duplex.*`/`memory.*` template). A stop+deny is
   recorded twice: the verdict AND the deny event.
4. **Bounds are labels**: a bounds formula is private-by-default state
   (№349/№355 lattice) — materializing it outside the verified contour is
   a compile error.

### 2.5 The stage-C injection-proof plan (№358, behind the gate)

The formula set is proven on an INJECTION CORPUS: for every monitored
signal and every bound, the corpus contains traces with (a) the bound
violated at a known instant, (b) the bound violated only under noise at
the declared ε boundary, (c) the bound satisfied. The proof: the monitor's
verdict matches the corpus annotation 100% — no missed violations (false
negatives are safety failures), no spurious violations at ε-boundary
noise (false positives erode the operator's trust and train people to
ignore the stop). The corpus is generated against the stage-C telemetry
shape; the harness and the runbook are №358's contract (behind the
GPU-budget gate — this ADR fixes the proof's DEFINITION, not its run).

## 3. Honest boundaries

1. **No hardware path exists** — stage C is a plan, not a component; the
   real-weights gate (№294) and the GPU-budget gate both apply. Nothing in
   this ADR promises hardware readiness.
2. **No STL parser/evaluator ships in this naryad** — №355 lands the type
   surfaces; the monitor implementation lands with the stage-A work (№356,
   behind the gate). This ADR is the decision record.
3. **The bound vocabulary is conservative** — the STL fragment supported
   at №356 is bounded-horizon, metric operators over scalar/pose signals;
   full STL (nesting unbounded until, quantitative semantics beyond the
   robustness degree) is out of scope and loud if ever requested.
4. **CBF/shielding and Simplex are prior art, not dependencies** — the
   monitor is a language-level runtime-verification contour; no control-
   theory solver enters the tree (the dependency discipline).

## 4. Consequences

- The sim investment (№356+) is hardware-portable: formulas and monitors
  carry over; only telemetry adapters (№360) are new.
- The stop+deny path reuses the existing enforcement surfaces (grants
  №340, DenyEvent №342, ledger №393/№415) — no new enforcement mechanism.
- The verifier's stage acceptance is mechanical: the §2.3 criteria are
  checkable against traces and ledger records, not narratives.
- The type surfaces (№355) must carry the bounds-formula and Proof
  contracts exactly as fixed here; a divergence is a returned naryad.

## 5. Links

- Registry §16.7 (В5): №354 (this ADR) → №355 (types) → №356–№361 (stages
  A/B/C, E-stop, ROS/backends, dispatch — behind the GPU-budget gate).
- Plan §9.2 (embodied handles), §9.3 (stage criteria), §17 (prior art:
  runtime verification/STL, CBF/shielding, Simplex).
- ADR-0114 (opaque handles), ADR-0154 (labels/consent), ADR-0155 (grant
  algebra, §3.5 the irreversible-action path), ADR-0167/№393/№415 (Action
  Ledger), №340 (revocation), №342 (DenyEvent(SimFailed)), №343 (signed
  Proof traces), №349 (the duty compile rule — the bounds are private).
