---
name: calibration
description: Discipline of probability quantification and Brier score tracking. Every forecast probability must be defensible, falsifiable, and contribute to the calibration record. The skill enforces Sherman Kent's verbal-to-numeric scale, computes Brier scores at verification, distinguishes calibration error from analysis error, tracks calibration by depth level and by fruit type to identify systematic biases. The feedback loop that makes ОСП learn rather than just opine.
---

# Calibration — The Feedback Loop That Makes Analysis Honest

A forecaster without calibration discipline produces opinions. A forecaster with calibration discipline produces predictions that compound — each prediction tested, each error categorized, each subsequent prediction informed by accumulated evidence about the forecaster's own biases.

The Brier score is the universal scoring function. Sherman Kent's calibrated language is the universal verbal-to-numeric translation. Together they prevent the most common forecasting failures: vague language disguising overconfidence, untestable predictions evading verification, anecdotal "I knew it would happen" replacing systematic learning.

This is the skill that makes the difference between an analyst with 100 forecasts and a calibrated forecaster with measurable improvement over time.

## Prerequisites

- Forecast produced (with explicit probabilities)
- Indicators defined (falsifiable, with dates)
- Archive infrastructure available (Analysis model with verification fields)
- Recognition that calibration is a discipline of the analyst, not just a metric

## Core principle

> Probability without calibration is theater. Probability with calibration is information. The Brier score doesn't measure how good your guess was; it measures how well your stated confidence matched reality across many predictions.

The mistake is to think calibration is about being right or wrong. Calibration is about whether your "70% confident" predictions actually come true 70% of the time across hundreds of cases. A forecaster who says "70%" and is right 70% of the time is well-calibrated. A forecaster who says "95%" and is right 70% of the time is overconfident. Both might have the same direct hit rate; only one is providing useful information.

## Sherman Kent verbal-to-numeric scale

All probability statements in ОСП analyses must use this calibrated scale (verbal terms map to specific numeric ranges):

| Verbal term | Numeric range |
|---|---|
| Almost certain / virtually certain | 95-99% |
| Highly likely | 80-94% |
| Likely / probable | 60-79% |
| Roughly even chance | 40-59% |
| Unlikely / improbable | 20-39% |
| Highly unlikely | 5-19% |
| Almost no chance / nearly impossible | 1-4% |

**Mandatory:** every probability statement in analysis includes both verbal term AND numeric range. "Likely" alone is insufficient. "60-79%" alone is acceptable. "Likely (60-79%)" is preferred — combines both formats for clarity.

**Forbidden:** vague terms not on the scale. "Possible," "could happen," "may occur," "significant chance" — these are unfalsifiable and must be replaced with calibrated terms.

## The Brier score

For binary predictions (event will / will not happen):

Brier score = (predicted_probability - actual_outcome)²

Where actual_outcome is 1 (event happened) or 0 (event didn't happen).

Examples:
- Predicted 70% probability event happens, event happens: (0.70 - 1)² = 0.09
- Predicted 70% probability event happens, event doesn't happen: (0.70 - 0)² = 0.49
- Predicted 50% probability, either outcome: (0.50 - X)² = 0.25

Range: 0 (perfect) to 1 (worst possible). Average across many predictions is the calibration metric.

For multi-class predictions (one of several scenarios), Brier generalizes to:
Brier = (1/N) × Σ (predicted_i - outcome_i)²

Where N is the number of scenarios, predicted_i is probability assigned to scenario i, and outcome_i is 1 for the actual scenario and 0 otherwise.

## Calibration targets

For ОСП analyses:
- Year 1: average Brier ≤ 0.25
- Year 2: average Brier ≤ 0.20
- Year 3: average Brier ≤ 0.18 (approaching expert-level)

For comparison: random forecast would average ~0.50 Brier on most distributions. Naive baseline (always predict 50%) averages ~0.25. Expert forecasters in Tetlock's studies achieved 0.15-0.18 over time.

## The procedure

### Step 1 — Pre-publish probability check

Before any analysis is finalized, review every probability statement:

**Check 1 — Calibrated language?**
Each probability uses Sherman Kent term and/or numeric range. No vague terms.

**Check 2 — Numeric defensibility?**
Why this number, not 5% higher or lower? Reasoning must be specific. "Probability 60% because [specific factors with relative weights]." Not "probability 60% because it feels likely."

**Check 3 — Falsifiable?**
Each probability attached to a specific event or scenario with specific indicators and dates. "Likely the situation will improve" — not falsifiable. "60-79% probability that X-indicator reaches Y-threshold by Z-date" — falsifiable.

**Check 4 — Implicit probabilities?**
Sometimes analyses make implicit probability claims through language ("the obvious outcome," "the inevitable result"). Make these explicit with calibrated terms.

### Step 2 — Probability range, not point estimate

Every probability is stated as a range, not a point. Single-point probabilities ("70%") are pseudo-precision. Reality has uncertainty about uncertainty.

Best practice: state center and range together: "70% (range 60-80%)". The range reflects:
- Quality of underlying data (better data → narrower range)
- Confidence in causal model (better understanding → narrower range)
- Time horizon (longer → wider range)
- Reflexivity considerations (reflexive predictions → wider range)

### Step 3 — Distinguish confidence levels

Three different things are sometimes conflated:

**Probability** — likelihood that prediction is correct
**Confidence** — how solid is the analysis underlying the probability
**Resolution** — how precise is the prediction (binary vs detailed scenario)

These are independent. A high-probability prediction can have low confidence (lots of factors, hard to weigh). A low-probability prediction can have high confidence (rare event, well-understood mechanism). High-resolution predictions sacrifice some accuracy for specificity.

Each forecast should state all three explicitly:
- Probability: 70% (range 60-80%)
- Confidence: medium (data quality good, causal model uncertain)
- Resolution: medium-high (specific event with date, but several pathways)

### Step 4 — Verification trigger

When indicator dates arrive (or upon `/verify <id>`):

For each indicator, determine:
- Did the indicator outcome match prediction? (binary or graded)
- What's the specific value? (e.g., ITA ETF reached +12% vs predicted ≥+15%)

Compute Brier component:
- Binary indicator: (predicted_probability - 1_or_0)²
- Range indicator: how close to predicted range? Map to binary (within range = 1, outside = 0)
- Multi-class indicator: which scenario actualized? Apply multi-class Brier

### Step 5 — Error categorization

Every miss is categorized:

**Tail event:** rare event happened, predicted at low probability and was correctly identified as low probability — calibration was correct, just rare event hit. No methodology change needed.

**Miscalibrated:** probability was wrong (too high or too low). Methodology may be sound but assumed wrong starting point.

**Analysis error:** the underlying analysis missed something — wrong forces identified, wrong roots, wrong topology. Methodology fix needed.

**Unfalsifiable:** the prediction was ambiguous, can't be cleanly verified. Quality gate failure — should have been caught pre-publish.

**Methodology gap:** the prediction was right at the leaf level but the methodology missed deeper structural issues (this category was added based on real analysis errors — see worked example below).

**Environmental misjudgment:** the prediction correctly identified roots, forces, and topology, but missed substrate depletion, climate shift, hidden disease, or parasite extraction. The tree looked healthy in the analysis; its environment was failing or it was hollowing internally; outcome diverged from prediction because environment changed faster than expected. Detection: for misses in this category, the gap is usually traceable to either (a) substrate scoring missed depletion trajectory, (b) internal diseases were not flagged in awareness frame, or (c) parasites were not identified through system-health-diagnostics. Methodology fix: tighten environmental assessment in awareness frame, lower thresholds for triggering system-health-diagnostics.

### Step 6 — Aggregate analysis

At end of each month, aggregate verifications:

**Overall Brier score** for the month, and trend across months.

**Brier by depth level:**
- Level 0 average
- Level 1 average
- Level 2 average
- Etc.

If higher depth levels don't show better Brier than lower, depth isn't producing value — methodological problem.

**Brier by fruit type:**
- Already-ripe fruits (probability of continued production)
- Ripening fruits (probability of full ripening)
- Forming fruits (probability of forming)
- Buds (probability of becoming fruits)
- Seeds (germination probability)

If specific fruit types show systematically high Brier, that fruit-type analysis is weak.

**Brier by topic class:**
- Geopolitical
- Macro financial
- Specific markets
- Personal/decision-maker analysis
- Etc.

Identifies systematic weaknesses by domain. Triggers Tier 2 skill review or Кузница debrief.

### Step 7 — Calibration curve

Plot predicted probability vs actual frequency:

- Group predictions by predicted probability bucket (e.g., 0-10%, 10-20%, ..., 90-100%)
- For each bucket, compute fraction that actually happened
- Plot: x-axis = predicted probability, y-axis = actual frequency
- Perfect calibration: diagonal line y = x

Common patterns:
- **Overconfidence:** high-probability predictions don't happen as often as predicted (curve below diagonal at high end)
- **Underconfidence:** low-probability predictions happen more than predicted (curve above diagonal at low end)
- **Compressed:** all predictions cluster near 50% regardless of stated probability (curve flatter than diagonal)

Calibration curve makes systematic biases visible. Adjustments target these patterns.

### Step 8 — Feedback to methodology

Calibration patterns drive specific methodology improvements:

- **Overconfidence at high probabilities** → tighten quality gates on "highly likely" predictions, require more skeptical Path B in two-paths-synthesis
- **Underconfidence at low probabilities** → may be missing tail risks, expand tail-event consideration
- **Worse Brier on specific topic class** → load specific Tier 2 skills more aggressively for that class, or add new skills via Кузница
- **Worse Brier at higher depth** → depth scaling isn't producing value, depth thresholds may need adjustment
- **Methodology gap category recurring** → specific methodology missing, requires Кузница intervention

## Worked example

**Past analysis #002:** Belarus + BYN/USD forecast (May 2026, this conversation).

### Original probabilities:
- Topic A (regime stability through 2026):
  - A. Stable, Lukashenko formally in power: 75% (highly likely 80-94 minus narrowed)
  - B. Health crisis / transit started: 15% (highly unlikely 5-19)
  - C. Sudden death/break: 5% (almost no chance 1-4 minus narrowed)
  - D. Mass protests with regime risk: 5%
- Topic B (USD/BYN at 2026-12-31):
  - A. 3.10-3.40: 55% (roughly even)
  - B. 2.80-3.10: 25% (unlikely)
  - C. 3.40-3.80: 15% (highly unlikely)
  - D. ≥3.80: 5%

### Methodological gap caught (per user feedback):
The Topic B forecast missed Russian fiscal cascade as primary driver. This is a **methodology gap** category error — not miscalibration, not analysis error in narrow sense, but missing a structural component (cross-country macro spillover) that the methodology should have flagged.

### Action taken:
Two methodology improvements identified for Кузница:
1. Add "cross-country macro spillover" sub-section to `narrative-vs-flows` skill
2. Add "cross-border Cantillon" section to `cantillon` skill

This is exactly what calibration discipline is for — converting methodology gaps into specific skill updates rather than vague "we'll do better next time."

### Confidence revision:
Per the gap, original confidence on Topic B should have been "medium" not "medium-high." Future BYN forecasts should explicitly include Russian macro section before being assigned medium-or-better confidence.

### Verification at 2026-12-31:
- Topic A: still pending. If Lukashenko formally in power, A scenario verified, Brier components for each scenario computed.
- Topic B: depending on actual BYN/USD on 2026-12-31, scenario verified.

The verification will populate Brier score with real numbers, and methodology gap will be tracked as additional metadata.

## Anti-patterns

- **Verbal probabilities only.** "Likely" without numeric anchor. Different people interpret "likely" as 50% or 80% — can't aggregate.
- **Single-point probabilities.** "70%" without range. False precision; reality has uncertainty about uncertainty.
- **Probability without falsifiable indicator.** "70% probability of significant change" — significant change to whom, by when, measured how? Without indicator, can't verify.
- **Defensive probabilities at 50%.** Risk-averse forecaster pulls all predictions toward 50% to avoid being wrong. Calibration curve will show compression. Brier suffers — this is how forecasters game without learning.
- **Overconfident on confirmed worldview.** Forecaster predicts what they want to be true at 90% probability. Calibration curve at high end will deviate from diagonal. Pattern visible only across many predictions.
- **No error categorization.** Verifying that prediction was wrong without categorizing why is missed learning. The five categories (tail, miscalibrated, analysis error, unfalsifiable, methodology gap) drive different fixes.
- **Calibration without methodology feedback.** Tracking Brier without using it to update skills. Calibration that doesn't improve methodology is theater.

## Output template (per-analysis)

```
─── CALIBRATION (per-analysis pre-publish check) ───

Calibrated language check: [pass | fail — list violations]
Numeric defensibility check: [pass | fail]
Falsifiability check: [pass | fail]
Implicit probabilities found and made explicit: [yes/no, list]

Per-prediction calibration:
- Prediction: <name>
  Probability: <value (range)>
  Confidence: [low | medium | high]
  Resolution: [low | medium | high]
  Reasoning for probability: <specific>

Total falsifiable indicators created: <count>
All have specific dates: [yes/no]
```

## Output template (monthly aggregate review)

```
─── CALIBRATION REVIEW (monthly) ───

Period: <month>
Total verifications: <count>
Average Brier: <value>
Trend vs prior month: <improving | stable | deteriorating>
Trend vs target: <ahead | on target | behind>

Brier by depth level:
- Level 0: <average over N predictions>
- Level 1: <average>
- ...

Brier by topic class:
- Geopolitical: <average over N predictions>
- Macro financial: <average>
- ...

Brier by fruit type:
- Already-ripe: <average>
- Ripening: <average>
- ...

Calibration curve summary:
- Overconfidence at high probabilities: [yes | no | mild]
- Underconfidence at low probabilities: [yes | no | mild]
- Compression: [yes | no | mild]

Error category breakdown:
- Tail events: <count>
- Miscalibrated: <count>
- Analysis errors: <count>
- Unfalsifiable: <count>
- Methodology gaps: <count>
- Environmental misjudgments: <count>

Methodology improvements identified:
- <Specific update to skill X based on pattern Y>
- ...

Top 1-2 priorities for next month:
- <Specific action>
```

## Integration with deconstruction protocol

This skill operates in two modes:

**Pre-publish mode:** invoked at end of every deconstruction (after all phases) before publishing. Verifies calibration discipline of the analysis.

**Post-verification mode:** invoked at indicator dates (or `/verify`) to compute Brier and categorize errors.

**Monthly review mode:** invoked at month-end to aggregate, identify patterns, drive methodology improvements.

This skill is the **learning loop** of ОСП. Without it, forecasts accumulate but methodology doesn't improve. With it, every forecast contributes to making future forecasts better.

Output stored in Analysis record (calibration metadata per analysis), AnalysisVerification record (verification details), MonthlyReview record (aggregated patterns). Drives Кузница debrief recommendations when patterns indicate methodology gaps.
