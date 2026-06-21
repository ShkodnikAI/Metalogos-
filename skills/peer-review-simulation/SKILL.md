---
name: peer-review-simulation
description: Simulates how reviewers from top journals (Nature, Science, NEJM, Physical Review Letters, etc.) would evaluate the project's technical claims. Uses standard peer review heuristics — methodology rigor, statistical validity, comparison to prior art, reproducibility, conflict of interest checks. Brings academic-grade scrutiny to commercial pitches, identifying issues that VC-style evaluation often misses.
---

# Peer Review Simulation — Academic Scrutiny On Commercial Claims

Top scientific journals reject 90%+ of submissions. Reviewers spend hours dissecting methodology, comparing claims against the literature, demanding reproducibility, checking for confounds. This level of scrutiny rarely reaches commercial pitches — VCs evaluate based on team, market, story. The project that wouldn't survive academic review can still get funded.

This skill applies the academic framework to evaluate technical claims. Result: identifying weaknesses that commercial-due-diligence misses, often the same weaknesses that emerge as project failures years later.

## Prerequisites

- `rapid-domain-immersion` providing field-specific peer review norms
- Project has technical claims with empirical content (not pure conceptual claim)
- Reviewer simulation appropriate for project type (different fields have different standards)

## Core principle

> Peer review is the highest filter scientific claims pass through. Most commercial pitches contain claims that wouldn't pass it. Simulating peer review surfaces methodological issues that determine whether a result is real or artifact, replicable or one-off, generalizable or contextual. Academic rigor finds different failures than commercial diligence — both matter.

## Reviewer perspectives by field

Different fields have different review priorities. Apply the perspective relevant to the project:

**Physical sciences (PRL, Nature Physics, Physical Review):**
- Theoretical consistency
- Experimental error analysis
- Reproducibility by independent groups
- Comparison to existing measurements/predictions
- Systematic vs statistical errors

**Life sciences (Nature, Cell, NEJM, JAMA):**
- Sample size and statistical power
- Controls and confounds
- Methodology details (could another lab replicate?)
- Comparison to standard of care / literature
- Conflict of interest disclosure
- Pre-registration of hypothesis

**Computer science (top venues — NeurIPS, SIGCOMM, IEEE):**
- Benchmark validity
- Comparison to baselines
- Ablation studies
- Generalization beyond test conditions
- Code/data availability

**Engineering (specific journal standards):**
- Failure mode analysis
- Operating envelope characterization
- Long-term reliability data
- Manufacturing reproducibility
- Standards compliance

**Medical / clinical (NEJM, Lancet, BMJ):**
- Endpoint selection and pre-specification
- Patient selection bias
- Blinding and randomization
- Adverse event reporting completeness
- Generalizability to broader populations

## Universal review questions

Regardless of field:

**Methodology:**
- Is methodology sufficient to support claim?
- Are controls appropriate?
- Are confounds addressed?

**Statistics:**
- Is sample size adequate for claimed effect size?
- Are statistical methods appropriate?
- Is multiple testing corrected for?
- Is variance reported?

**Comparison to prior art:**
- Are similar prior results cited?
- Are differences from prior art technical or methodological?
- Is comparison to current SOTA fair?

**Reproducibility:**
- Could another lab/team replicate this?
- Are protocols documented sufficiently?
- Is data available?

**Generalization:**
- Beyond exact conditions tested, what claims are valid?
- Are claims appropriately scoped?

**Conflicts:**
- Who funded the work?
- What's the relationship between authors and outcomes?
- Are competing approaches treated fairly?

## The procedure

### Step 1 — Identify field and review standard

Different fields have different rigor standards. Apply perspective matching project's nominal field. For interdisciplinary projects, apply both/multiple perspectives.

### Step 2 — Extract testable empirical claims from materials

What specific empirical claims does the project make? Distinguish:
- Theoretical claims (conceptual; reviewed for consistency)
- Empirical claims (data-supported; reviewed for methodology)
- Forecast claims (predictions; reviewed for grounding)

Focus mainly on empirical claims for review simulation.

### Step 3 — For each empirical claim, run review

For each claim, ask the universal questions plus field-specific questions. Note where claim:
- Is well-supported by evidence shown
- Has methodology gaps preventing strong conclusion
- Has comparison/baseline issues
- Has reproducibility concerns
- Has generalization issues

### Step 4 — Identify critical gaps

For top empirical claims, identify the gaps that would cause peer review rejection. These are the critical concerns — even if not blocking commercially, they should be probed.

### Step 5 — Convert to combat questions

Reviewer-style questions formulated for meeting context. Often these are the most penetrating questions because they probe methodology rather than conclusions.

### Step 6 — Output peer review report

Structured output with critical gaps and meeting questions.

## Worked example — fictional fusion startup

**Empirical claims:**
1. Q=10 in 2027
2. 30-second pulse achieved in their lab
3. Cost reduction projection of 30% per doubling

**Claim 1 review (Q=10 in 2027):**

This is a forecast not yet empirical. Review for grounding:
- Methodology: scaled extrapolation from current 30-second pulse to commercial parameters
- Comparison: ITER targets Q=10 with much larger machine, no one has demonstrated yet
- Reproducibility: extrapolation depends on plasma physics holding at scale
- Generalization: jump from achieved (0.5-1 second at low temperature) to claimed (continuous at high temperature) is large

**Critical gap:** several orders of magnitude extrapolation without sufficient intermediate validation milestones.

**Reviewer question:** "What intermediate plasma conditions have you achieved that scale toward Q=10? What's the next 10x improvement and when will it be demonstrated?"

**Claim 2 review (30-second pulse):**

This is empirical:
- Methodology: was pulse stable or required active stabilization throughout?
- Comparison: similar pulses achieved by ITER predecessor JET, EAST in China
- Statistics: was this single shot or reproducible across many shots? variance?
- Generalization: at what temperature, density, current — were these commercial-relevant or relaxed?

**Critical gap:** unclear whether their 30-second is at parameters that scale to commercial vs at relaxed parameters that don't.

**Reviewer question:** "What were the specific plasma parameters of your 30-second shot — temperature, density, current? How do they compare to commercial requirements?"

**Claim 3 review (cost reduction trajectory):**

Forecast not empirical:
- Methodology: typical learning curve assumption (Wright's Law)
- Comparison: precedent for similar reductions in other fusion or analogous tech
- Generalization: assumes deployment volume sufficient for learning to occur

**Critical gap:** learning curve requires scale; first 5-10 plants may not see cost reduction.

**Reviewer question:** "Wright's Law produces cost reduction at scale; what scale is required for your 30% per doubling assumption? Plant 1 to plant 10 — what's the cost trajectory?"

## Anti-patterns

- **Wrong field standard.** Applying CS reviewer standards to biotech misses key concerns.
- **Pure theoretical without empirical focus.** Peer review is most powerful on empirical claims — that's where rigor differentiates.
- **Reviewer pose without rigor.** Sounding skeptical isn't review. Specific methodology questions are.
- **Missing reproducibility lens.** Commercial diligence rarely asks "could another team replicate this?" Peer review centers it.
- **Ignoring conflict of interest.** Funder relationships, advisor interests, competing claims by same authors elsewhere — peer review notices, commercial diligence often skips.
- **Generalization blindness.** Extrapolating beyond tested range is universal soft spot. Reviewer-grade scrutiny catches.

## Output template

```
─── PEER REVIEW SIMULATION ───

Project: <identifier>
Field standards applied: <list>

EMPIRICAL CLAIMS IDENTIFIED:
1. <Claim>
2. <Claim>

REVIEW PER CLAIM:

Claim 1: <description>
Methodology assessment: <strengths/weaknesses>
Statistical assessment: <strengths/weaknesses>
Comparison to prior art: <fair/missing/biased>
Reproducibility: <demonstrable/unclear/no>
Generalization: <appropriate/overstated>
Conflicts: <disclosed/concerning/unclear>
Verdict: [accept | major revision | reject in academic terms]
Critical gap: <specific issue>

[More claims]

CRITICAL GAPS IDENTIFIED:
- <Gap 1>: implications
- <Gap 2>: implications

MEETING QUESTIONS:
1. <Reviewer-style question for combat list>
2. <Question>
[3-5 typical]
```

## Integration with Expert protocol

Tier 2 — invoked when:
- Project makes substantial empirical claims
- Field has strong peer review tradition
- High-stakes evaluation justifies rigor

Output integrates into combat questions and bullshit detection.

Stored in ExpertBriefing under `peerReviewSimulation`.

Particularly valuable for academic spinouts (where founders may not yet have made claims through peer review) and biotech (where regulatory approval requires peer-review-grade evidence).
