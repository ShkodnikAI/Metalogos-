---
name: norm-base-fixation
description: Fixing the normative base — which SP / GOST / Eurocode / PUE / IEC standards govern a calculation — before any number is computed. A calculation without a stated norm is not verifiable and has no engineering force. The norm determines load factors, safety coefficients, and methods; choosing it is not a formality, it is the foundation of the calculation.
---

# Norm Base Fixation — No Calculation Without a Stated Norm

An engineering calculation is meaningless without knowing which rules it follows. The same roof, calculated under different norms, gives different required member sizes — because the norms prescribe different load factors, different safety coefficients, different methods. Fixing the norm base is the first step of every calculation, before a single number.

## Prerequisites

- An engineering task with a known discipline (structural / hydraulic / power electronics)
- Loaded for every calculation — Tier 1

## Core principle

> The norm is not a citation added at the end to look proper — it is the framework that determines every load factor, every safety coefficient, and every method used. Choosing the norm is choosing the rules of the calculation. A number computed without a fixed norm is not a calculation; it is an estimate that cannot be checked, cannot be approved, and has no engineering force.

## Why this is hard rule 2

A calculation without a stated norm fails in three ways:
- **Not verifiable** — a reviewing engineer cannot check it; they don't know what rules it should follow.
- **Not approvable** — a licensed engineer cannot sign a calculation whose normative basis is unknown.
- **Not comparable** — two calculations under different norms cannot be compared; the numbers mean different things.

So the norm base is fixed *before* calculation begins, and recorded explicitly.

## The norm base is a parameter, not a constant

The department does not hard-wire one country's norms. The norm system is a **parameter** of each task. Default — Russian Federation. Supported — international systems. The task fixes which applies.

**Russian Federation (default):**
- Structural: SP 20.13330 (loads and actions), SP 16.13330 (steel structures), SP 64.13330 (timber structures), SP 70.13330 (load-bearing structures), SP 17.13330 (roofs)
- Hydraulic: GOST standards for hydraulic drives; sector norms for lifting equipment
- Power electronics: PUE (electrical installation rules); GOST for electrical safety and power electronics

**International (on selection):**
- Structural: Eurocode 0 (basis of design), Eurocode 1 (actions), Eurocode 3 (steel), Eurocode 5 (timber)
- Hydraulic: ISO standards for hydraulic fluid power
- Power electronics: IEC standards (e.g. for inverters and photovoltaic systems)

If the task does not specify, the default (RF) is used — and that default is stated explicitly in the output, so the reviewing engineer knows and can object if the project actually needs a different system.

## The procedure

### Step 1 — Identify the discipline and object
Structural / hydraulic / power electronics, and the object type (roof, greenhouse, lift, inverter, etc.). The discipline narrows which norms are relevant.

### Step 2 — Determine the norm system
RF by default. If the task, the location, or the client indicates otherwise (a project in another country, a client requiring Eurocode), use that — and confirm the choice is deliberate.

### Step 3 — Select the specific norms
Not "the building codes" — the specific documents. For a steel greenhouse roof under RF norms: SP 20.13330 for the loads (snow, wind), SP 16.13330 for the steel members, SP 17.13330 for roofing specifics. Name each and what it governs.

### Step 4 — Check norm currency
Norms get revised. Use the current edition. If unsure whether an edition is current, flag it — `NormReference` tracks currency, and the semi-annual norm review keeps it updated. A calculation under a superseded norm is a real error class.

### Step 5 — Record the norm base
Write it into the calculation explicitly: each norm code, edition, and what it governs. This populates `normsApplied` on `EngineeringCalculation`. No norm base recorded → the calculation cannot proceed past `drafted`.

### Step 6 — Note norm-driven parameters
The norms prescribe load factors, combination rules, safety coefficients. Extract the ones this calculation needs *now*, from the fixed norms — so the calculation uses norm values, not remembered or assumed ones.

## Mixing norm systems — avoid it

A calculation uses one norm system. Mixing — RF load factors with Eurocode member checks — produces an incoherent result, because the systems are internally calibrated (a Eurocode safety factor assumes Eurocode load factors). If a project genuinely requires elements from different systems, that is a decision for the licensed engineer, flagged explicitly, not done silently by the department.

## Worked example

Task: check the load-bearing capacity of a greenhouse roof.

**Step 1 — discipline / object:** structural mechanics; object — greenhouse roof, light steel frame.

**Step 2 — norm system:** no country specified → default RF. Stated in the output: "Norm system: Russian Federation (default — no jurisdiction specified; reviewing engineer to confirm applicability)."

**Step 3 — specific norms:**
- SP 20.13330 — loads and actions (governs snow load, wind load, dead load determination)
- SP 16.13330 — steel structures (governs the strength/stability checks of the steel members)
- SP 17.13330 — roofs (governs roofing-specific requirements)

**Step 4 — currency:** editions checked against `NormReference`; current editions used. (If an edition were uncertain, the output would flag: "edition currency to be confirmed.")

**Step 5 — recorded:** the three norms written into `normsApplied` with code, edition, and scope.

**Step 6 — norm-driven parameters:** from SP 20.13330 — the snow load for the region, the load combination factors; from SP 16.13330 — the material safety factor for the steel grade. These are extracted now, from the norms, so the calculation uses them rather than guessed values.

Only now — with the norm base fixed and recorded — does `load-case-enumeration` and the actual calculation begin.

## Anti-patterns

- **Calculating without a norm.** A number with no stated normative basis. Unverifiable, unapprovable — the thing this skill exists to prevent.
- **Norm as afterthought.** Computing first, then adding a norm citation to look proper. The norm must drive the calculation, not decorate it.
- **Vague norm reference.** "Per building codes" — which codes, which editions? Name the specific documents.
- **Superseded edition.** Using an old edition of a norm that has been revised.
- **Silent default.** Using the RF default without stating that it is a default — the reviewing engineer must know a jurisdiction wasn't specified.
- **Mixing norm systems.** RF load factors with Eurocode checks. Incoherent.
- **Remembered coefficients.** Using a safety factor "from memory" instead of extracting it from the fixed norm. The norm's actual value governs.
- **Assuming the norm.** Picking norms without confirming they fit the object and jurisdiction.

## Output template (norm base block — part of every calculation)

```
NORM BASE
Discipline: structural | hydraulic | power_electronics
Object: <object type>
Norm system: RF | Eurocode | IEC | ISO   [if default RF used: "(default — jurisdiction not specified)"]

Norms applied:
- <code> (<edition>) — <what it governs>
- <code> (<edition>) — <what it governs>
[...]

Norm-driven parameters extracted:
- <parameter> = <value> <unit>  — from <norm code>
[...]

Edition currency: confirmed current | flagged for confirmation
```

This populates `normSystem` and `normsApplied` on `EngineeringCalculation`. The calculation cannot leave `drafted` without it.

## Integration

- Tier 1 — loaded for every calculation
- Runs before `load-case-enumeration` and any Tier 2 calculation skill
- `NormReference` model stores the norm catalogue and currency
- The semi-annual norm review keeps `NormReference` current
- `wrong_norm` is a tracked error class in `EngineeringCalculationVerification`
- `engineering-disclaimer-discipline` — the disclaimer plus a fixed norm base is the minimum a calculation must carry
