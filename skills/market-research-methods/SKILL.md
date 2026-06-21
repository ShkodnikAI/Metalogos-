---
name: market-research-methods
description: Methods for researching markets and audiences with explicit data quality grading. Behavioral data beats surveys beats expert opinion beats assumption. Every research output names its sources and their reliability. Turns "I think the market wants X" into a defensible research finding.
---

# Market Research Methods — Findings, Not Opinions

"Research" in marketing is too often a confident opinion with no data underneath. Real research names where every claim came from and how trustworthy that source is. This skill is how the department produces findings that survive scrutiny.

## Prerequisites

- A concrete research question (not "study the market")
- `audience-segmentation` available — research often defines or validates segments

## Core principle

> A research finding is only as strong as its weakest source. State the source and its reliability for every claim, or it isn't research — it's a guess wearing a lab coat. The honest researcher would rather say "we don't know, here's how to find out" than fabricate confidence.

## The data quality hierarchy (hard rule 8)

Sources, strongest to weakest:

1. **Behavioral data** — what people actually *did*. Purchase records, usage logs, what converted, what churned, A/B test results. People's actions are honest; their words often aren't. **Highest reliability.**

2. **Direct observation** — watching real people use a product or face the problem. Usability sessions, support tickets, recorded interactions. Strong, though small-sample.

3. **Surveys and interviews** — what people *say*. Useful for understanding motivation and language, but people misremember, rationalize, and tell you what they think you want. **Medium reliability** — and weaker for predicting behavior than for understanding it.

4. **Expert opinion** — informed judgment from someone who knows the domain. Useful for direction, weak for specifics. **Low-medium reliability.**

5. **Assumption** — reasoned guess with no data. Sometimes unavoidable. Must be labeled as such. **Lowest — flag explicitly.**

Every research output tags each finding with its source tier. A finding built on assumptions is not wrong to include — but it must be visibly marked so decisions account for the uncertainty.

## Start with the research question

Bad research starts with "study the market". Good research starts with a specific, answerable question:

- ✗ "Research our market."
- ✓ "How do solo business operators currently track the state of their business, and what frustrates them most about it?"
- ✓ "What would a solo operator pay per month for a tool that consolidates their scattered information?"
- ✓ "Which channels do our converting users come from, and at what cost each?"

A sharp question makes the research bounded and verifiable. A vague question produces a vague report nobody can act on.

## Research types

**Market sizing** — how big is the opportunity? How many people have this problem, how much do they spend on alternatives. Honest sizing gives a range and shows the math.

**Audience study** — who are the people, what's the job-to-be-done, what's the pain, the language, the alternatives. Feeds `audience-segmentation`.

**Competitive** — who else serves this need, how, at what price, with what weaknesses. See `competitive-analysis`.

**Channel analysis** — where the audience is, which channels reach them, at what cost. Feeds `channel-strategy`.

**Trend scan** — what's shifting in audience behavior, platforms, formats. Feeds the semi-annual channel review.

## Methods toolkit

**Analyzing existing behavioral data:**
- Your own usage/conversion/churn data — the richest source you have, often underused
- Public data — platform statistics, published reports, industry numbers
- Search and community signal — what people search for, what they complain about in forums/communities (this is behavioral — real questions people actually asked)

**Talking to people:**
- Interviews — open-ended, for understanding motivation and language. 5-10 good interviews reveal patterns; don't need hundreds.
- Surveys — structured, for quantifying. Beware: leading questions produce the answer you wanted. Beware: people predict their own future behavior badly.
- The key interview question is about the *past*, not the future: "Tell me about the last time you faced this problem" beats "Would you use a tool that...". Past behavior is data; predicted behavior is fiction.

**Observation:**
- Watching real usage, reading support tickets, reviewing recorded sessions
- The gap between what people *say* they do and what they *actually* do is where the real insight lives

**Synthesis:**
- Triangulate — a finding confirmed by behavioral data AND interviews AND observation is solid; a finding from one weak source is tentative

## The "would you use it" trap

The most common research mistake: asking people if they'd use/buy something. They say yes to be nice, or because imagining is easy and acting is hard. This "research" produces false confidence and failed launches.

Replace future-hypothetical questions with past-behavioral ones:
- ✗ "Would you pay for a tool that consolidates your business info?"
- ✓ "What do you currently use to keep track? Walk me through the last time it let you down. Have you ever paid for anything to fix this?"

The past-behavior version produces data. The future-hypothetical version produces wishful noise.

## The procedure

### Step 1 — Define the question
One specific, answerable research question. Write it down. Everything serves it.

### Step 2 — Choose methods
Pick methods that fit the question. Prefer behavioral sources. Use interviews/surveys to *understand*, behavioral data to *predict*.

### Step 3 — Gather, tagging sources
As findings accumulate, tag each: source + reliability tier.

### Step 4 — Triangulate
Cross-check findings across sources. Note where sources agree (solid) and disagree (investigate).

### Step 5 — Produce forecasts
Where the research supports it, state numeric forecasts (segment size, expected conversion, willingness to pay) — these become verifiable later.

### Step 6 — State confidence and gaps
Overall confidence (high/medium/low). Explicitly list what's still unknown and how to find out. Honest gaps beat false completeness.

### Step 7 — Set verification date
The numeric forecasts get a date when reality can check them. Research, like campaigns, is verified.

## Worked example

Research question: "How do solo business operators currently track the state of their business, and what would make them switch to a dedicated tool?"

**Methods used:**
- Analyzed support conversations from existing fosved-bot users (behavioral — high reliability)
- 8 interviews with solo operators, past-behavior focus (medium reliability)
- Scanned 4 small-business Telegram communities for recurring complaints (behavioral signal — medium-high)

**Findings (tagged):**
- "Solo operators keep information in 3-5 disconnected places on average" — interviews + support data, medium-high
- "The trigger to seek a tool is a specific painful failure (missed something important), not gradual frustration" — interviews, medium
- "Willingness to pay clusters around a low monthly figure; above it, resistance spikes" — interviews, medium (note: stated WTP is weak; flag as needing a real pricing test)
- "Estimated segment size: several thousand reachable via the 4 communities" — community subscriber counts, medium

**Forecasts (verifiable later):**
- A consolidation tool, well-positioned, could convert ~2-4% of reached community members — confidence: low (no direct precedent)

**Confidence:** medium overall. **Gaps:** real willingness-to-pay needs a pricing test, not interview claims; channel effectiveness untested.

**Verification date:** set for after the first campaign provides real conversion data.

Note what this does NOT do: it doesn't claim "the market definitely wants this" or invent a precise market size. It states what's known, how well, and what isn't.

## Anti-patterns

- **Opinion as research.** "I think the market wants X" with no sources. The thing this skill exists to prevent.
- **No source tagging.** Findings with no indication of where they came from or how reliable.
- **"Would you use it" questions.** Future-hypothetical questions producing wishful noise.
- **Leading survey questions.** Questions written to get the answer you hoped for.
- **Survey worship.** Treating stated preferences as behavioral fact.
- **No research question.** "Study the market" — unbounded, produces an unactionable report.
- **False completeness.** Hiding the gaps to look thorough. Honest gaps are strength.
- **Single-source findings presented as solid.** One weak source, stated confidently.
- **Research with no forecasts.** If it produces no verifiable predictions, it can't be checked or learned from.
- **Ignoring own behavioral data.** Running surveys while your own usage logs sit unexamined.

## Output template

```
MARKET RESEARCH — <title>

Research question: <the specific question>
Product context: <which Fosved product / external client>

METHODS
<methods used, and why each>

FINDINGS (each tagged: source | reliability)
- <finding> — <source>, <high/medium/low>
- ...

FORECASTS (verifiable)
- <metric>: <predicted value/range> — basis: <...>, confidence: <...>

OVERALL CONFIDENCE: high | medium | low

GAPS — what is still unknown
- <gap> — how to resolve: <...>

VERIFICATION DATE: <when the forecasts can be checked>
```

This populates the `MarketResearch` model.

## Integration

- Tier 2 — loaded for research-mode tasks
- Feeds `audience-segmentation` (validates/defines segments)
- Feeds `competitive-analysis`, `channel-strategy`
- Forecasts verified later, like campaigns
- `/market-research` command runs this procedure
