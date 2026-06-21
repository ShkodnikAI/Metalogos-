---
name: power-electronics-inverter
description: Preparing power-electronics calculations for inverters and switching equipment — power and current ratings, voltage levels, component stresses, protection, thermal sizing. For converting and switching power from wind generators and solar panels. Power-electronics discipline block. Output is preliminary and requires a licensed engineer's review.
---

# Power Electronics — Inverter and Switching Equipment

This skill prepares the calculation behind an inverter or a switching unit (commutator): the power and current it must handle, the voltage levels, the stress on each component, the protection, the heat to dissipate. For converting and switching power from renewable sources — wind generators and solar panels — to a usable load or grid. It produces a documented preliminary calculation for a licensed engineer to review.

## Prerequisites

- `norm-base-fixation` complete — electrical norms fixed (PUE, IEC for inverters/PV)
- `load-case-enumeration` complete — the electrical operating and fault cases
- `engineering-disclaimer-discipline` governs the output

## Core principle

> A power-electronics device is rated not by the power it usually handles but by the worst case every component must survive — peak current, peak voltage, fault current, the temperature at full load on the hottest day. An inverter sized for the average is under-rated for the moments that destroy it. And electrical failure is not just non-function — it is fire. The calculation is paranoid about peaks, faults, and heat because the failure mode is dangerous.

## Why electrical work carries fire risk

Structural failure crushes; electrical failure can also *burn*. An undersized component overheats; an unprotected fault arcs; an inverter without proper thermal design degrades and fails. So the power-electronics block carries the same paranoia as the structural block, pointed at: peak stresses, fault currents, protection, and heat. A device that "works on the bench" but has no margin for the fault case or the hot day is not designed.

## What the calculation covers

**Power and current ratings.** The device must handle the rated power — and the peak. For renewables the input is variable: a solar array's output swings with irradiance; a wind generator's with wind speed. The inverter is rated for the *maximum* the source can deliver, not the average.

**Voltage levels.** Input voltage range (a PV string's voltage varies with temperature and irradiance — the cold-clear-day open-circuit voltage is a notorious peak), output voltage, the voltage every component sees. Components are rated above the worst-case voltage with margin.

**Component stresses.** Each switching device, capacitor, inductor, conductor — the current through it, the voltage across it, in normal operation and at the peak. Every component rated for its worst case.

**Fault cases.** Short circuit, overload, source or load fault. The fault current, and what limits it. Protection — fuses, breakers, the inverter's own protection — sized to clear faults before damage.

**Thermal.** Power electronics dissipate heat. At full load, on the hottest expected ambient, does every component stay within its temperature limit? Heatsinking, airflow, derating. Thermal under-design is a slow failure that ends in degradation or fire.

**Protection and safety.** Over-current, over-voltage, over-temperature protection. For grid-connected — anti-islanding and the grid-code requirements. Earthing/grounding per the electrical norms. Isolation.

## Load and operating cases specific to power electronics

From `load-case-enumeration`, the electrical cases:

- **Rated operation** — the device at nameplate conditions
- **Peak source output** — the maximum the wind/solar source can deliver (cold clear noon for PV; high wind for a turbine)
- **Peak voltage** — PV string open-circuit voltage on a cold clear day; the highest voltage components see
- **Overload** — above rated, and how long it is tolerated
- **Short circuit / fault** — fault current magnitude, clearing
- **Thermal worst case** — full load at maximum ambient temperature
- **Startup / transient** — inrush, switching transients

## The procedure

### Step 1 — Define the system
Source (wind generator / solar array — and its characteristics: power range, voltage range, variability), the load or grid, what the inverter/switch must do between them.

### Step 2 — Rate for the peaks
Power rating ≥ the peak the source can deliver. Voltage ratings ≥ the worst-case voltages (including the PV cold-day open-circuit peak). Current ratings ≥ the peak current. Rate for the worst case, not the average — with margin.

### Step 3 — Component stresses
For each component — switching devices, capacitors, inductors, conductors — the current and voltage it sees in normal operation and at the peak. Select each component rated above its worst case with margin.

### Step 4 — Fault analysis
The fault cases — short circuit, overload. The fault current. The protection — fuses, breakers — sized and coordinated to clear faults before components are damaged.

### Step 5 — Thermal design
Power dissipated at full load. At the maximum ambient temperature, does every component stay within limits? Heatsinking, airflow, component derating for temperature. Thermal margin confirmed.

### Step 6 — Protection and safety
Over-current, over-voltage, over-temperature protection. Earthing per the norm. Isolation. For grid-tie — grid-code compliance, anti-islanding. The electrical-safety norm (PUE / IEC) requirements met.

### Step 7 — Verdict, verification, documentation
Verdict: a coherent design where every component survives every case with margin and protection clears faults / or a gap stated plainly. `independent-verification` — the power = voltage × current relations, the thermal estimate, dimensional checks cross-check. Then `calculation-documentation`.

## The renewable-source peculiarity

Wind and solar are not steady supplies. This shapes the calculation:

- **Solar:** output swings from zero to peak with irradiance; string voltage *rises as temperature falls* — the highest voltage occurs on a cold, clear day, possibly at low power. The inverter's voltage rating must cover this cold-day peak, which is easy to miss because it does not coincide with peak power.
- **Wind:** output swings with wind speed; gusts cause transients; the turbine may need to be limited or dumped in high wind.

The inverter is rated for the *envelope* of what the source can do — every corner of it — not for a typical operating point. The detailed system matching (string sizing, MPPT range, source-to-inverter fit) is the subject of `renewable-system-matching`; this skill rates the inverter/switch itself for the resulting stresses.

## Worked example (structure of the calculation)

Task: an inverter for a solar array feeding a load. Norms fixed (RF — PUE; IEC for the inverter and PV). Cases enumerated.

**Step 1 — system:** the array's power range and voltage range (including the cold-day open-circuit voltage), the load, the conversion required.

**Step 2 — rate for peaks:** power rating ≥ the array's peak output; the inverter's maximum input voltage ≥ the array's cold-day open-circuit voltage (the peak that does not coincide with peak power); current ratings ≥ peak current. Margins applied.

**Step 3 — component stresses:** for the switching devices, the DC-link capacitors, the inductors, the conductors — the current and voltage each sees at the peak. Components selected rated above with margin.

**Step 4 — faults:** short-circuit current; protection (fuses/breakers) coordinated to clear it before damage; the inverter's internal protection.

**Step 5 — thermal:** dissipation at full load; at the maximum ambient temperature, every component within its limit; heatsinking and derating confirmed.

**Step 6 — protection/safety:** over-current/voltage/temperature protection; earthing per PUE; isolation; grid-code items if grid-connected.

**Step 7 — verification + documentation:** cross-check the relations and the thermal estimate, document, disclaimer, reviewing engineer.

The numbers depend on real source data, real components, real norms. Output is preliminary — a licensed electrical engineer reviews and approves.

## Anti-patterns

- **Rating for the average.** Sizing for typical power, not the peak the source can deliver. Under-rated for the moments that matter.
- **Missing the PV cold-day voltage peak.** The highest string voltage occurs cold, not at peak power — easy to miss, and it can exceed the inverter's voltage rating.
- **No fault analysis.** Designing for normal operation, ignoring short-circuit and overload. No protection coordination.
- **Thermal ignored.** Sizing electrically but not thermally. Components within electrical limits can still overheat — slow failure, fire risk.
- **Thermal at average ambient.** Checking heat at a mild temperature, not the hottest expected day.
- **No protection / under-protection.** Missing over-current, over-voltage, over-temperature protection, or earthing per norm.
- **Treating the source as steady.** Designing for one operating point, not the variable envelope of wind/solar.
- **Grid-code ignored** for grid-tie systems (anti-islanding, grid requirements).
- **Non-standard components.** Specifying parts that don't exist.
- **Treating the output as final.** Preliminary. A licensed electrical engineer reviews and signs.

## Output

Produces the calculation content for `calculation-documentation`. Populates `EngineeringCalculation`: `method`, `resultSummary` (ratings + component set), `resultDetails`, `safetyFactor` (margins), `verdict`. Discipline = `power_electronics`.

## Integration

- Tier 2 — power-electronics block; loaded for inverter / switching tasks
- Built on `norm-base-fixation` + `load-case-enumeration`
- `renewable-system-matching` handles source-to-system fit (string sizing, MPPT, energy balance); this skill rates the device for the resulting stresses
- `independent-verification` + `calculation-documentation` complete it
- Output preliminary — `engineering-disclaimer-discipline` and engineer review apply
