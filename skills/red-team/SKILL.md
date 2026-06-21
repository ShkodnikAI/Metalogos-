---
name: red-team
description: Tier 3 special technique. Builds an alternative analysis from the perspective of an adversary attempting to refute, exploit, or invert the conclusions of an existing ОСП analysis. Activated for high-stakes decisions where premature convergence on a single view carries significant risk. Reveals blind spots, unstated assumptions, and exploitation vectors invisible to the analyst who built the original analysis.
---

# Red Team — The Adversarial Mirror

The most dangerous moment in any analysis is when the analyst becomes confident in their conclusions. Confidence stops the search for disconfirming evidence. The original analysis, no matter how rigorous, was built by someone who knew its conclusions before they finished — confirmation bias is structural in single-author work.

Red Team is the structured corrective. It assigns the role of attacker — someone whose job is to refute, undermine, exploit, or invert the original analysis. Not to fairly evaluate it; to break it. The discipline is adversarial because adversarial is the missing perspective.

In military planning, Red Team is institutionalized — separate teams whose only job is to defeat plans. In financial firms, dedicated risk teams play this role. For ОСП, it's a Tier 3 skill — invoked for high-stakes analyses where convergence on a single view without testing carries real cost.

## Prerequisites

- Existing ОСП analysis with non-trivial stakes
- Time and discipline to do the inversion properly (not perfunctory)
- Recognition that this is criticism, not consensus-building

## Core principle

> An analysis that has not been attacked is an analysis that has not been tested. The analyst who built it cannot fully attack it — confirmation bias is structural. Adversarial perspective is the missing input. Without it, blind spots compound.

The mistake is to think Red Team is "considering counterarguments." Counterarguments are nuance. Red Team is hostile reconstruction — building a different analysis whose conclusions invert or undermine the original.

## What Red Team does (and doesn't)

**Does:**
- Build alternative analysis from same evidence
- Identify unstated assumptions in original
- Find evidence that would refute original
- Identify exploitation vectors against the original conclusions
- Argue for opposite scenarios with full rigor

**Does not:**
- Fairly evaluate the original (that's review, not red-teaming)
- Find compromise positions (red-teaming is adversarial)
- Validate the original (validation is yes-set, red-teaming is no-set)
- Propose action recommendations (red-teaming surfaces problems; action is owner's choice)

## The procedure

### Step 1 — Receive original analysis

Read the original analysis carefully. Note:
- Stated conclusions
- Stated probabilities
- Key supporting evidence
- Identified scenarios
- Falsifiable indicators

### Step 2 — Identify unstated assumptions

Most analyses have assumptions the analyst didn't realize they were making. Find them:

- What's treated as fixed that could change?
- What's treated as obvious that could be wrong?
- What's treated as continuing that could end?
- What ranges are stated as if they cover possibilities (and what's outside the ranges)?

For each unstated assumption, ask: if this assumption is wrong, how does the analysis change?

### Step 3 — Build inverted scenarios

For the major scenarios in the original analysis, construct the inverse case:

- Original says X is likely → Red Team builds case for not-X
- Original says actor A will do Y → Red Team builds case for actor A doing not-Y
- Original predicts trend continues → Red Team builds case for trend reversal

For each inverted scenario, build the case as rigorously as possible. Use the same evidence; argue different interpretation.

### Step 4 — Find evidence the original didn't use

Sometimes evidence exists that would have refuted the original analysis if considered. Active search for:

- Data the original analysis didn't cite
- Sources with contrary views and credentials
- Historical analogues with different outcomes
- Recent events the original might have missed
- Adversarial sources making contrary claims

### Step 5 — Identify exploitation vectors

If the original analysis is published or acted upon, how could an adversary exploit it?

- What would adversary do differently knowing this analysis is public belief?
- What deceptions could exploit the analyst's framework?
- What positions could adversary take advantage of?
- What information warfare could be conducted to shape the analyst's future inputs?

This is a different lens than scenario inversion — it asks specifically about exploitation, not just alternatives.

### Step 6 — Identify reflexive impacts

Drawing on `reflexive-impact` skill: if the analysis is acted upon, does its action change the conditions it analyzed?

- Self-fulfilling: action makes analysis come true
- Self-defeating: action makes analysis fail
- Both can be exploited by aware adversary

### Step 7 — Construct alternative high-confidence forecast

From all the above, build the strongest possible forecast that contradicts the original:

- Different probabilities on the major scenarios
- Different fruits with different beneficiaries
- Different time horizons
- Different confidence levels
- Different recommended actions

This is not a "second-best forecast" — it's the strongest contrary forecast that can be constructed from available evidence.

### Step 8 — Compare and synthesize

Place original and Red Team forecasts side by side. For each significant divergence:

- Which is more strongly supported?
- What evidence would resolve the disagreement?
- What's the probability the Red Team is right?

The synthesis is not "average of two forecasts." It's "original forecast adjusted by what Red Team revealed":
- Confidence on points Red Team didn't successfully challenge: maintained or strengthened
- Confidence on points Red Team challenged but couldn't refute: somewhat weakened
- Conclusions Red Team successfully refuted: revised
- Blind spots Red Team revealed: now explicit
- Exploitation vectors Red Team identified: now defended against

## Worked example — Belarus regime stability forecast

**Original ОСП analysis:** 75% probability of regime stability through 2026. Supports: cycle position late-stage but not collapsed; Russian patron support; institutional continuity.

**Red Team analysis:**

**Unstated assumptions to challenge:**
- Russian patron support assumed to continue at current level (could fail if Russian fiscal crisis intensifies)
- Lukashenko health assumed adequate (could fail with sudden incident)
- Elite cohesion assumed (could fragment under specific stress)
- No unstated assumption about Western re-engagement

**Inverted scenarios:**

Scenario "Stability through 2026 fails":
- Russian fiscal crisis escalates faster than expected → patron support contracts
- Specific Lukashenko health event creates succession scramble
- Inner circle defection (someone with knowledge defects to West)
- External event (war in Ukraine ends in Russian setback) destabilizes Russian patron capacity
- Combination of above → 30-40% probability of significant regime stress events through 2026, not 25%

**Evidence the original might have missed:**
- Specific health indicators in recent Lukashenko appearances suggesting accelerated aging
- Patterns of inner circle members positioning for post-Lukashenko era visible in real estate purchases, family relocations
- Russian internal documents (leaked) suggesting Russian planners considering "managed Belarus transition"

**Exploitation vectors:**
- If our analysis (75% stability) becomes consensus, opportunities for asymmetric bets on instability are mispriced
- Defense and security firms positioning for Belarus contingency are early-stage opportunities
- Currency-related bets (long USD/BYN with specific structure) are asymmetric if instability materializes

**Reflexive impacts:**
- Our confidence in stability could lead to under-positioning for instability scenarios
- If multiple analysts converge on similar view, market mispricing creates opportunity
- Stability forecast doesn't change reality; reality depends on specific events independent of forecasts

**Red Team alternative forecast:**
- Probability of significant regime stress events through 2026: 35-45% (vs original 25%)
- Probability of formal regime change through 2026: 15-25% (vs original 10%)
- Confidence: lower than original — situation more volatile than original projects

**Synthesis:**
- Original forecast probably too confident in stability
- Adjusted forecast: 65% stable through 2026 (down from 75%); 25% significant stress; 10% formal change
- Watchlist additions: specific Lukashenko health indicators, specific Russian patron commitment indicators
- Investment opportunities: asymmetric positions on instability are underpriced if original consensus holds

This Red Team revision substantially changes the forecast and identifies actionable mispricing.

## Anti-patterns

- **Polite Red Team.** Soft criticism that doesn't actually challenge. Discipline: hostile reconstruction, not friendly review.
- **Strawman Red Team.** Constructing weak alternative views to refute. Real Red Team builds strongest contrary case from same evidence.
- **Red Team without action.** Identifying problems with original analysis but not adjusting conclusions. The point is to revise, not to acknowledge weakness while maintaining position.
- **Red Team as veto.** Treating Red Team as deciding factor when its job is to surface considerations. Original analysis still holds where Red Team didn't successfully challenge.
- **Same-analyst Red Team.** Original analyst playing Red Team on own work. Confirmation bias too strong; structural blindness persists. Different person if possible; explicit time-separation if not.
- **Red Team as ritual.** Going through motions without genuine adversarial effort. The cost is the value — half-effort red-teaming produces half-value insight.

## Output template

```
─── RED TEAM ANALYSIS ───

Original analysis: <reference>
Red Team conducted: <date, conditions>

UNSTATED ASSUMPTIONS IDENTIFIED:
- <Assumption 1, with implication if wrong>
- <Assumption 2>
[3-7 typical]

INVERTED SCENARIOS:
For each major scenario in original:
- Original: <claim>
- Inverted: <full alternative case>
- Evidence supporting inversion: <list>

EVIDENCE NOT USED IN ORIGINAL:
- <Item 1, with implications>
- <Item 2>
[may be empty if original was thorough]

EXPLOITATION VECTORS:
- <How adversary could exploit if original analysis is public>
- <Deceptions that would work given original framework>

REFLEXIVE IMPACTS:
- Self-fulfilling tendencies: <list>
- Self-defeating tendencies: <list>
- Adversary exploitation of reflexivity: <list>

ALTERNATIVE FORECAST (strongest contrary):
- Probabilities revised: <comparison>
- Time horizons revised: <comparison>
- Confidence revised: <comparison>
- Recommended action implications: <comparison>

SYNTHESIS:
Points where original held: <list>
Points where original weakened: <list with extent>
Points refuted: <list with full revision>
Blind spots now explicit: <list>
New watchlist additions: <list>

REVISED FINAL FORECAST:
<Adjusted version of original incorporating Red Team findings>
```

## Integration with ОСП

Tier 3 — invoked for:
- Owner request via `/redteam <analysis_id>`
- Auto-trigger for any analysis with high stakes (depth 4 + significant capital implications)
- Quarterly review of analyses where ОСП calibration suggests high-confidence positions need testing

When invoked, ideally conducted by separate process or with explicit time-separation from original analysis.

Output stored in Analysis record under `redTeam` field. Synthesis becomes the operational forecast going forward.

This skill, when applied seriously, prevents the most expensive analytical errors — the ones produced by confidence in conclusions that should have been challenged. It costs effort. It earns its cost when it changes a single high-stakes decision.
