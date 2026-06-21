---
name: load-case-enumeration
description: Enumerating every load case and load combination a structure or device must withstand before calculating — permanent, live, snow, wind, operational, emergency. A missed load case is a missed path to failure: the calculation can be perfect for the cases considered and the structure still collapses under the one that wasn't. The paranoid core of structural and mechanical safety.
---

# Load Case Enumeration — Every Path to Failure, Listed First

A structure does not fail under the loads you calculated for. It fails under the load you forgot. Load-case enumeration is the disciplined listing of every load, every combination, every scenario the object must survive — done *before* calculation, so nothing is calculated against an incomplete picture.

## Prerequisites

- `norm-base-fixation` complete — the norms prescribe which load cases apply and how they combine
- The object and its service conditions are known

## Core principle

> The calculation can be flawless for every case on the list and the structure can still collapse — if the governing case was not on the list. Completeness of the load-case list is therefore more important than precision of any single calculation. This is the paranoid step: before computing anything, ask relentlessly "what else could load this, and what could combine with what".

## Why this is hard rule 3, and why it is the paranoid step

The department's psychotype is Paranoid + Pedant. The paranoia lives here. A missed load case is the single most dangerous error class — more dangerous than a wrong coefficient, because a wrong coefficient usually still gets the order of magnitude right, while a missed case means a whole failure path was never checked at all.

So before any number: enumerate. Exhaustively. Suspiciously. "What else."

## Load types to enumerate

The norms (fixed in `norm-base-fixation`) define the categories. Generally:

**Permanent loads** — always present:
- Self-weight of the structure
- Weight of permanent equipment, finishes, roofing

**Live / variable loads** — present sometimes:
- Occupancy / use loads (people, stored goods)
- Equipment that may or may not be there
- Maintenance loads (a person walking on a roof to clean it)

**Environmental loads:**
- Snow load — depends on region, roof shape, drift, accumulation in valleys
- Wind load — depends on region, height, shape, suction on roofs
- Temperature effects — expansion, contraction
- Ice load where relevant
- Rain load, ponding on flat roofs

**Operational loads** (for devices — lifts, drives):
- Working load, rated load
- Dynamic loads — acceleration, deceleration, impact
- Cyclic loads — fatigue from repeated operation

**Emergency / accidental loads:**
- Overload scenarios
- Failure of a component
- Extreme environmental events beyond normal service

## Load combinations — loads act together

Loads do not occur one at a time. The norm prescribes **combinations** — which loads are assumed simultaneous, and with what factors. A roof must survive dead load + full snow + wind suction *together*, not each alone.

The governing case is often a combination, not a single load. Enumerate the combinations the norm requires, not just the individual loads.

The norm also gives **combination factors** — not every load is taken at its maximum simultaneously (it is statistically unlikely that peak snow and peak wind coincide), so the norm reduces some. Use the norm's factors; do not invent them.

## The paranoid questions

For every object, ask — deliberately, suspiciously:

- What loads are obviously there? (self-weight, rated load)
- What loads are there *sometimes*? (snow, occupancy, maintenance)
- What loads are there *rarely*? (extreme weather, accidental)
- What makes the environmental loads *worse*? (snow drift into a valley, wind suction on an overhang, ponding on a flat roof)
- What loads act *together*, and which combination is worst?
- What happens during *operation*, not just at rest? (dynamic, cyclic, impact)
- What happens if something *fails*? (a support, a component)
- What did the norm's combination list include that I haven't thought of?

The job is not done when you have a plausible list. It is done when you have asked "what else" and the honest answer is "nothing within the norm's scope".

## The procedure

### Step 1 — Identify the object's service conditions
Where it is (region — for snow/wind), how it is used, how long it must last, what operates on or in it.

### Step 2 — Enumerate by type
Walk every load type above. For each: is it present for this object? If yes, list it with a provisional magnitude basis.

### Step 3 — Apply the norm's combination rules
From the fixed norms, list the load combinations that must be checked. Each combination is a separate case to calculate.

### Step 4 — Apply combination factors
For each combination, the norm's factors — which loads at full value, which reduced.

### Step 5 — Identify worsening conditions
Snow drift, wind suction, ponding, accumulation — the geometry-driven amplifications. These are easy to miss and often govern.

### Step 6 — Mark the likely governing case(s)
Which combination is probably critical. The calculation will confirm — but flag it now so attention goes there.

### Step 7 — Record the complete list
Every load case and combination written into `loadCases` on `EngineeringCalculation`. The calculation proceeds against this list; nothing is calculated outside it.

## Worked example

Object: greenhouse roof, light steel frame, in a snowy region. Norms fixed (RF: SP 20.13330 for loads).

**Permanent:** self-weight of steel frame; weight of glazing/covering.

**Variable:**
- Snow load — significant in this region. Worsening condition: the greenhouse has a valley between two roof slopes → **snow drift accumulation in the valley** — this is a separate, higher local case, easily missed.
- Maintenance load — a person on the roof for cleaning/repair.

**Environmental:**
- Wind load — pressure on windward, **suction on leeward and on overhangs**. For a light roof, wind suction can be a governing uplift case — the roof lifting off, not pressing down.
- Temperature — steel expansion/contraction; relevant for a long frame.

**Operational:** none (static structure).

**Emergency:** extreme snow beyond the normal service value — per norm's accidental combination if applicable.

**Combinations to check (per SP 20.13330):**
1. Dead + full snow (uniform)
2. Dead + snow with valley drift accumulation
3. Dead + wind suction (uplift — the light-roof danger)
4. Dead + snow + wind, with the norm's combination factors
5. Maintenance load case (dead + person, on the relevant member)

**Likely governing:** case 2 (valley drift) for the valley members; case 3 (wind uplift) for the connections and light members. Flagged for attention.

Note what enumeration caught that a careless approach misses: the **valley drift** (a geometry-driven amplification) and the **wind uplift** (a light-roof failure mode where the danger is the roof leaving, not the roof sagging). Both are classic missed cases. Both are now on the list, so both will be calculated.

## Anti-patterns

- **Calculating before enumerating.** Jumping to numbers with an incomplete load picture. The forgotten case is not calculated.
- **Single loads, no combinations.** Checking dead, snow, wind separately but never together. The governing case is usually a combination.
- **Ignoring worsening geometry.** Missing snow drift in valleys, wind suction on overhangs, ponding on flat roofs.
- **Forgetting uplift.** For light structures, wind suction (the roof lifting off) is often more critical than downward load. Easy to forget if you only think "loads push down".
- **Inventing combination factors.** Using made-up factors instead of the norm's. The norm's factors are calibrated; yours are not.
- **Skipping operational loads.** For devices — forgetting dynamic, impact, and cyclic/fatigue loads; calculating only the static rated load.
- **Skipping accidental cases.** Not checking the norm's accidental/emergency combinations.
- **Stopping at "plausible".** Ending the list when it looks reasonable, instead of asking "what else" until the honest answer is "nothing".
- **Not flagging the governing case.** Not marking which combination is likely critical, so attention is not focused.

## Output template (load-case block — part of every structural/mechanical calculation)

```
LOAD CASES & COMBINATIONS
Object: <object>  |  Service conditions: <region, use, life>
Norms governing load determination: <from norm-base-fixation>

INDIVIDUAL LOADS
Permanent:
- <load> — basis: <...>
Variable:
- <load> — basis: <...>   [note worsening conditions: drift, suction, ponding]
Environmental:
- <load> — basis: <...>
Operational (devices):
- <load> — basis: <...>
Emergency / accidental:
- <load> — basis: <...>

COMBINATIONS TO CHECK (per <norm>)
1. <combination> — factors: <...>
2. <combination> — factors: <...>
[...]

LIKELY GOVERNING CASE(S): <which, and why> — flagged for attention

Completeness check: "what else could load this?" asked — answer: <nothing further within norm scope / + additional cases found>
```

This populates `loadCases` on `EngineeringCalculation`.

## Integration

- Tier 1 — loaded for every structural and mechanical calculation
- Runs after `norm-base-fixation` (norms define combinations) and before any Tier 2 calculation
- `structural-load-bearing` and `hydraulic-drive-design` calculate against this list
- `missed_load_case` is a tracked — and the most serious — error class in `EngineeringCalculationVerification`
- `missed-load-case rate` is a department metric (target < 10%)
