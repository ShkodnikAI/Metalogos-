---
name: hydraulic-drive-design
description: Preparing hydraulic drive calculations — cylinders, pressures, flows, pump sizing — for hydraulic lifts and actuators. Determining the forces, the working pressure, the flow for the required speed, and component selection. Hydraulics discipline block. Output is preliminary and requires a licensed engineer's review.
---

# Hydraulic Drive Design — Force, Pressure, Flow

This skill prepares the calculation behind a hydraulic drive — most commonly a hydraulic lift: what force is needed, what pressure and cylinder produce it, what flow gives the required speed, what pump and components suit. It produces a documented preliminary calculation for a licensed engineer to review.

## Prerequisites

- `norm-base-fixation` complete — hydraulic / lifting-equipment norms fixed
- `load-case-enumeration` complete — the loads the drive must move and hold
- `engineering-disclaimer-discipline` governs the output

## Core principle

> A hydraulic drive is a chain — load to cylinder to pressure to flow to pump — and the calculation must hold every link. Sizing the cylinder for the static load alone misses the dynamic load of acceleration, the pressure losses along the line, the holding case when the load must not creep down. A lift that raises the load but cannot hold it, or cannot start it moving, is not designed — only half-calculated.

## The hydraulic chain

A hydraulic drive calculation follows a chain of dependent quantities:

**Load → Force required.** What the drive must move — and the load cases: static weight, the dynamic addition from accelerating it, friction, and the holding case (the load stationary, which the system must sustain without creep).

**Force → Pressure × Area.** Force = pressure × effective piston area. The cylinder bore and the working pressure together produce the force. Choosing them is a trade — higher pressure allows a smaller cylinder but stresses components more.

**Speed → Flow.** The required speed of motion sets the flow rate: flow = piston area × speed. Faster motion needs more flow.

**Flow → Pump.** The pump must deliver that flow at that pressure. Pump sizing follows from flow and pressure together.

**Losses along the way.** Pressure drops in lines, valves, fittings. The pump must overcome the working pressure *plus* the losses. Ignoring losses under-sizes the pump.

Every link must hold. A break anywhere — a cylinder too small, a pump with too little flow, unaccounted losses — means the drive does not perform.

## Load cases specific to hydraulic drives

Beyond the static load, the drive-specific cases (from `load-case-enumeration`):

- **Static load** — the weight to be moved, at rest
- **Dynamic load** — the extra force to accelerate the load to working speed; deceleration too
- **Friction** — seal friction, guide friction — resists motion, adds to the force needed
- **Holding case** — the load stationary at height: the system must hold it without creep, which concerns the valves, seals, and the locked column of fluid
- **Overload** — what happens above rated load; the relief valve setting
- **The lowering case** — going down, the load assists motion; control of the descent (not letting it run away) is its own concern

A lift calculation that covers only "raise the static load" has missed most of these.

## The procedure

### Step 1 — Define the duty
What is moved, how heavy, how far, how fast, how often. The duty cycle — frequent operation raises fatigue and heating concerns.

### Step 2 — Force required (all cases)
The force for each case: static, static + dynamic (acceleration), plus friction. The governing force is usually static + dynamic + friction together. The holding case is checked separately — it concerns holding, not moving.

### Step 3 — Cylinder and working pressure
Select a working pressure and a cylinder bore so that pressure × effective area meets the governing force, with margin. The effective area differs between extend and retract (the rod occupies area on the rod side) — both directions checked. Cylinder from standard ranges.

### Step 4 — Flow for the required speed
Flow = effective area × required speed. For both directions — the rod-side/bore-side area difference means extend and retract speeds differ for the same flow.

### Step 5 — Account for losses
Pressure losses in lines, valves, fittings. The pump's pressure must cover working pressure + losses. Estimate the losses; do not ignore them.

### Step 6 — Pump and components
Pump sized for the required flow at (working pressure + losses). Then the supporting components — relief valve (set above working, below component limits), control valves, holding/check valves for the holding case, lines sized for the flow without excessive loss.

### Step 7 — Holding and safety
The holding case: how the load is held without creep — check/holding valves, the locked fluid column. For a lift, the consequence of a holding failure is the load descending uncontrolled — so this case gets paranoid attention. Overload: the relief valve protects the system.

### Step 8 — Verdict, verification, documentation
Verdict: a coherent set of components that performs the duty across all cases / or a gap stated plainly. Then `independent-verification` (the force ↔ pressure×area ↔ flow relations cross-check well) and `calculation-documentation`.

## The holding case deserves special paranoia

For a hydraulic lift, the most dangerous failure is not "fails to raise" — it is "raises, then the load descends uncontrolled". A failure to raise is visible and harmless; a holding failure drops a load on whatever is below.

So the holding case is checked with extra suspicion: how is the load held — by holding/check valves, by a mechanically locked column? What happens if a seal leaks, if a valve fails, if a line bursts while the load is up? The norms for lifting equipment have specific requirements here. This is exactly the paranoid step the department's psychotype is for.

## Worked example (structure of the calculation)

Task: a hydraulic lift to raise a known load to a height. Norms fixed (RF — hydraulic and lifting-equipment norms). Load cases enumerated.

**Step 1 — duty:** the load weight, lift height, required raise speed, expected operating frequency.

**Step 2 — force:** static force = weight; dynamic addition for accelerating to raise speed; plus seal/guide friction. Governing raise force = static + dynamic + friction. Holding force = static (held, not moving).

**Step 3 — cylinder + pressure:** choose a working pressure; cylinder bore so that pressure × bore-side area ≥ governing raise force with margin. Check the retract direction on the rod-side area. Standard cylinder selected.

**Step 4 — flow:** flow = bore-side area × required raise speed. Note the retract speed for the same flow (rod-side area is smaller → faster retract).

**Step 5 — losses:** estimate pressure losses in the lines and valves between pump and cylinder.

**Step 6 — pump + components:** pump delivering the required flow at (working pressure + losses). Relief valve above working pressure and below component ratings. Holding/check valve for the holding case. Lines sized for the flow.

**Step 7 — holding + safety:** how the raised load is held without creep; what protects against uncontrolled descent on a component failure; relief valve for overload. Paranoid attention here — per the lifting-equipment norm.

**Step 8 — verification + documentation:** cross-check the chain, document fully, disclaimer, reviewing engineer.

The numbers depend on real inputs and norms. The output is preliminary regardless — a licensed engineer reviews and approves the lift design.

## Anti-patterns

- **Static load only.** Sizing for the weight at rest, ignoring the dynamic force of acceleration and friction. The drive then cannot start the load moving at speed.
- **Ignoring the holding case.** Calculating the raise, not how the load is held. A lift that cannot hold is dangerous.
- **Forgetting losses.** Sizing the pump for working pressure only, ignoring line/valve losses. The pump then under-delivers.
- **Same area both directions.** Forgetting that bore-side and rod-side effective areas differ — extend and retract forces and speeds are not equal.
- **No relief valve / wrong setting.** No overload protection, or a relief set wrong relative to working pressure and component limits.
- **Descent uncontrolled.** Not addressing the lowering case — the load assists motion and can run away without control.
- **Duty cycle ignored.** Frequent operation brings fatigue and fluid heating; a "single lift" calculation misses them.
- **Non-standard components.** Specifying a cylinder or pump no manufacturer makes.
- **Treating the output as final.** Preliminary. A licensed engineer reviews and signs.

## Output

Produces the calculation content for `calculation-documentation`. Populates `EngineeringCalculation`: `method`, `resultSummary` (the component set), `resultDetails` (the chain calculation), `safetyFactor`, `verdict`. Discipline = `hydraulic`.

## Integration

- Tier 2 — hydraulics block; loaded for hydraulic-drive tasks
- Built on `norm-base-fixation` + `load-case-enumeration`
- `independent-verification` (the chain relations cross-check) + `calculation-documentation` complete it
- Output preliminary — `engineering-disclaimer-discipline` and engineer review apply
- A drive that powers a structure may connect to a `structural-load-bearing` calculation of what it lifts/supports
