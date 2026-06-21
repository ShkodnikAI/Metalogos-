---
name: renewable-system-matching
description: Matching renewable sources to the rest of the system — solar string sizing and MPPT-range fit, wind generator to converter matching, energy balance, source-to-inverter compatibility. The system-level fit calculation that sits beside inverter rating. Power-electronics discipline block. Output is preliminary, requires a licensed engineer's review.
---

# Renewable System Matching — Source, Converter, Load in Balance

`power-electronics-inverter` rates the inverter for the stresses it sees. This skill is the system-level question that sits beside it: do the source, the converter, and the load actually *fit* each other? A perfectly-rated inverter is useless if the solar string's voltage falls outside its MPPT range, or if the energy the source produces does not balance what the load needs.

## Prerequisites

- `norm-base-fixation` complete — electrical norms fixed (PUE, IEC for PV/wind)
- `load-case-enumeration` complete — operating cases and environmental conditions
- Often paired with `power-electronics-inverter`
- `engineering-disclaimer-discipline` governs the output

## Core principle

> A renewable system works when three things fit: the source produces within the converter's input window across all conditions, the converter delivers what the load needs, and the energy balances over time. A mismatch at any of the three — a string voltage outside the MPPT range on a cold morning, a converter that clips the source's peak, a generation that does not cover the load — means a system that under-performs or does not function, even if every individual component is correctly rated.

## The three matching questions

### 1. Source-to-converter electrical fit

The source's output must stay inside the converter's input window — across *all* conditions, not just nominal.

For solar:
- **String voltage range.** A PV string's voltage varies with temperature and irradiance. It must stay within the inverter's operating window across the full temperature range — and the cold-day open-circuit voltage must not exceed the inverter's maximum input voltage. String length (panels in series) is chosen so the voltage envelope fits.
- **MPPT range fit.** The inverter's maximum power point tracking has a voltage range where it can actually track. The string's operating voltage must stay inside it across conditions, or the inverter cannot harvest the available power.
- **Current / power.** String/array current and power within the inverter's input ratings.

For wind:
- The generator's voltage and frequency (variable with wind speed) within what the converter can accept and process.
- Power range, including what happens in high wind (limiting, dumping).

### 2. Converter-to-load fit

The converter's output must match what the load or grid requires — voltage, frequency, phase, power quality. For grid-tie, the grid-code requirements. For an off-grid load, the load's actual demand profile.

### 3. Energy balance over time

Beyond instantaneous electrical fit — does the energy balance? Over a day, a season, a year: does the source generate enough to cover the load's consumption? For systems with storage, does the storage bridge the gaps (night, calm periods)? A system electrically correct but energy-short does not do its job.

## The procedure

### Step 1 — Characterize the source
The solar array or wind generator: power range, voltage range across temperature and conditions, the variability envelope. The corners matter — coldest, hottest, peak, zero.

### Step 2 — Characterize the load
What the load needs — voltage, frequency, power — and its demand profile over time. For grid-tie, the grid's requirements.

### Step 3 — Source-to-converter fit
Check the source's full output envelope against the converter's input window:
- String voltage (cold open-circuit ≤ inverter max; operating range within MPPT range) — size the string length accordingly
- Current and power within input ratings
- For wind — voltage/frequency envelope within the converter's acceptance
If the envelope does not fit, the string/array configuration changes, or the converter choice changes.

### Step 4 — Converter-to-load fit
Check the converter's output against the load/grid requirements. Voltage, frequency, power quality, grid-code items.

### Step 5 — Energy balance
Over the relevant cycle (day / season / year): generation vs consumption. With storage — does storage cover the gaps? Identify shortfalls or excess.

### Step 6 — Verdict
The system matches across all three questions / or a specific mismatch stated plainly (string outside MPPT range on cold days; energy shortfall in winter; converter cannot meet the load) — with what would need to change.

### Step 7 — Verification and documentation
`independent-verification` — the voltage envelope, the energy balance cross-check; `calculation-documentation`.

## The temperature-and-voltage trap (solar)

The most common renewable-matching error: sizing a string for nominal conditions and forgetting that **PV voltage rises as temperature falls**. The string that sits comfortably in the MPPT range at 25 °C can:
- exceed the inverter's maximum input voltage on a cold clear morning (open-circuit, cold) — potentially damaging the inverter
- or drift to the edge of the MPPT range so power is not tracked well

String length must be chosen against the *cold* extreme for the voltage ceiling and the *hot* extreme for the voltage floor. The whole temperature range, not the nominal point. This is the matching equivalent of `load-case-enumeration`'s "the case you forgot" — here the forgotten case is the cold morning.

## Worked example (structure of the calculation)

Task: match a solar array to an inverter for a given load. Norms fixed (RF — PUE; IEC for PV). Conditions enumerated, including the local temperature extremes.

**Step 1 — source:** the panels' characteristics; the voltage of one panel across the local temperature range (cold extreme → higher voltage, hot extreme → lower); current and power.

**Step 2 — load:** the load's voltage, power, demand profile; or the grid's requirements for grid-tie.

**Step 3 — source-to-converter fit:**
- String length: choose how many panels in series so that — at the coldest expected temperature, open-circuit — the string voltage stays below the inverter's maximum input voltage; and at the hottest, the operating voltage stays above the MPPT range floor. The string length is bounded above by the cold case and below by the hot case.
- Number of strings: so total current and power are within the inverter's input ratings.

**Step 4 — converter-to-load fit:** the inverter output meets the load/grid requirements.

**Step 5 — energy balance:** over the year, generation vs the load's consumption; seasonal shortfall (winter, low sun) identified; storage role if present.

**Step 6 — verdict:** the array configuration and inverter match across electrical fit and energy balance — or a stated mismatch (e.g. "string of N panels exceeds inverter max voltage on cold days — reduce to N-1" or "winter generation covers only part of the load — storage or a larger array needed").

**Step 7 — verification + documentation:** cross-check, document, disclaimer, reviewing engineer.

The numbers depend on real panel data, real temperature extremes, a real inverter. Output is preliminary — a licensed electrical engineer reviews and approves.

## Anti-patterns

- **Nominal-condition sizing.** Sizing the string at 25 °C and forgetting cold-day voltage rise. The classic, dangerous renewable-matching error.
- **Ignoring the MPPT range.** Fitting voltage within the inverter's absolute limits but outside its MPPT tracking range — power is left unharvested.
- **Electrical fit without energy balance.** A system that is electrically correct but generates less than the load needs. It "works" and still fails its purpose.
- **Forgetting storage gaps.** For off-grid — not checking that storage bridges night / calm periods.
- **Treating the source as steady.** Matching for one operating point, not the full variable envelope.
- **Wind high-speed case ignored.** Not addressing what happens in high wind (limiting, dumping).
- **Grid-code overlooked** for grid-tie systems.
- **Confusing matching with rating.** This skill is system fit; `power-electronics-inverter` is device rating. Both are needed; neither replaces the other.
- **Treating the output as final.** Preliminary. A licensed electrical engineer reviews and signs.

## Output

Produces the matching calculation for `calculation-documentation`. Populates `EngineeringCalculation`: `method`, `resultSummary` (the matched configuration — string sizing, inverter fit, energy balance result), `resultDetails`, `verdict`. Discipline = `power_electronics`.

## Integration

- Tier 2 — power-electronics block; loaded for renewable-system tasks
- Pairs with `power-electronics-inverter` — this skill does system fit, that one rates the device
- Built on `norm-base-fixation` + `load-case-enumeration` (environmental conditions, especially temperature extremes)
- `independent-verification` + `calculation-documentation` complete it
- Output preliminary — `engineering-disclaimer-discipline` and engineer review apply
