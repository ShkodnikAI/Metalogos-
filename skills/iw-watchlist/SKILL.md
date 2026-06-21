---
name: iw-watchlist
description: Indications & Warnings system — Pentagon-style structured early warning. For each major scenario or risk, define indicators that would appear at 30/60/90/180/365 days BEFORE the event becomes obvious. Watchlist is monitored continuously; accumulating indicators trigger alerts that escalate analysis depth. Converts ОСП from reactive (answers when asked) to active (raises alerts before crises become visible). The mechanism that catches Rheinmetall-before-Iran-war and similar asymmetric opportunities.
---

# I&W Watchlist — Catching Events Before They Are Obvious

The Pentagon's J2 maintains Indications and Warnings watchlists for major scenarios. For each scenario (war with country X, regime collapse in country Y, financial crisis triggered by event Z), an analyst has predefined indicators. These indicators are signals that would appear *before* the event becomes obvious to the public. The watchlist is monitored continuously; accumulating indicators trigger escalation.

This is what differentiates strategic intelligence from journalism. Journalism reports what has happened. Strategic intelligence catches what is *preparing to happen*. The difference between "stocks rose during the war" and "buy defense stocks before the war" is exactly this difference.

## Prerequisites

- ОСП is operational with Analysis archive
- Awareness that this is a **continuous** skill, not invoked per-analysis
- Owner has identified topics worth watching (or system has triggered watchlist additions automatically based on prior analyses)

## Core principle

> Most major events have observable preparation phases lasting weeks to months before the event itself. The events feel sudden because nobody was watching the right indicators. Define the indicators *now*; check them *continuously*; act when accumulation crosses threshold.

The mistake is to think major events are unpredictable. Most aren't. They look unpredictable to those who weren't watching. The professional analyst isn't smarter than the journalist; they're watching different indicators.

## What goes on the watchlist

A watchlist entry is a **scenario**, not just a topic. The difference matters.

**Topic (insufficient):** "China"
**Scenario (correct):** "Chinese decision to attempt military action against Taiwan within next 24 months"

The scenario is specific enough that you can ask "what would I see in advance if this is preparing?". A topic is too vague to generate predictive indicators.

Each watchlist entry defines:
- **Scenario:** specific event or transition with horizon
- **Threshold:** what does "happening" mean operationally (binary or graded)
- **Indicators at multiple time horizons:** 30 / 60 / 90 / 180 / 365 days before event
- **Alert thresholds:** how many indicators at what intensity trigger escalation
- **Current state:** which indicators are active right now

## The procedure

### Step 1 — Define scenarios for watchlist

Scenarios come from three sources:

**Source A — Owner priorities.** Topics owner explicitly cares about (geopolitical regions of interest, asset classes held, business decisions on horizon).

**Source B — Prior ОСП analyses.** When an analysis identifies a major potential event with non-trivial probability, add the event as a watchlist scenario for ongoing monitoring.

**Source C — Lab signals.** When Лаборатория знаний identifies a tracked technology approaching inflection, related scenarios go on the watchlist.

Aim for 5-15 active scenarios at any time. Less than 5 means watchlist is underutilized. More than 15 means dilution — too many to monitor properly.

### Step 2 — Define indicators per scenario

For each scenario, generate indicators at multiple time horizons. The discipline: **what specific observable would appear at this time horizon if the scenario is preparing?**

**T-30 indicators (30 days before event):**
The most operational indicators. Movement of forces, financial positioning, communications patterns, leadership statements.

**T-60 to T-90 indicators:**
Pre-positioning. Logistics, intelligence-collection patterns, third-party communications, financial flows.

**T-180 indicators:**
Strategic positioning. Doctrine changes, training shifts, public-information operations, alliance discussions.

**T-365 indicators:**
Background preparation. Personnel changes in key positions, R&D priorities, long-term resource allocation.

For each indicator, specify:
- What the observable is (specific, binary or graded)
- Where to look for it (data source, channel, public record)
- What baseline looks like (so deviation is detectable)
- What threshold constitutes "active"

### Step 3 — Establish baseline

For each indicator, establish what normal looks like. Without baseline, deviations cannot be detected.

Baseline can be:
- Historical pattern (what this indicator has shown over past X years)
- Comparable systems (what this indicator shows in similar contexts)
- Stated intent (what authoritative sources claim is normal)

Baseline must be specific enough that current measurement can be classified as: at baseline / above baseline / significantly deviating / sharply deviating.

### Step 4 — Set alert thresholds

For each scenario, define:

**Yellow alert:** N indicators at T-180+ horizon active simultaneously OR M indicators at any horizon at "deviating" level. Triggers ОСП level-2 (wargame) on the scenario.

**Orange alert:** Yellow conditions persist 30+ days OR T-90 indicators activate OR T-180 indicators move from "deviating" to "sharply deviating". Triggers ОСП level-3 (deep dive) on the scenario.

**Red alert:** T-30 to T-60 indicators activate OR multiple T-90 indicators sharply deviating. Triggers ОСП level-4 (full intelligence operation) on the scenario, immediate owner notification.

Thresholds are scenario-specific. A war scenario may have lower thresholds (high stakes, asymmetric cost of missing). A market scenario may have higher thresholds (more noise in indicators, more false positives).

### Step 5 — Monitoring infrastructure

Indicators are checked on schedule:

**Daily:** T-30 indicators for active alerts (red/orange status scenarios)
**Weekly:** T-60 to T-90 indicators for all scenarios
**Monthly:** T-180 indicators for all scenarios
**Quarterly:** T-365 indicators for all scenarios

This load is managed via scheduler — most checks are automated (parsing public sources for specific signals), some require manual research (specific channels, specific contacts).

### Step 6 — Indicator activation handling

When indicator activates:

1. Verify (was this signal real? false positive?)
2. Check baseline (is this actually deviating from normal, or is normal just noisy?)
3. Update scenario state in Watchlist DB
4. Recompute alert level for the scenario
5. If alert level changed, trigger appropriate ОСП analysis depth
6. Log indicator activation with date, source, intensity

### Step 7 — Periodic watchlist review

**Monthly:**
- Are all scenarios still relevant?
- Have any indicators proven uninformative (always active, never active, or no correlation with eventual outcomes)?
- Are there new scenarios worth adding?

**Quarterly:**
- Major review of all scenarios
- Retire scenarios that have resolved (event occurred, or window closed)
- Add new scenarios from ОСП analyses, Lab signals, owner requests
- Evaluate indicator quality — which indicators have been most predictive

### Step 8 — Owner alert protocol

For active alerts:

**Yellow:** weekly digest mentions scenario state
**Orange:** specific notification (Telegram message, not buried in digest) with current indicators and recommendation
**Red:** immediate notification with action recommendation

Alert notifications are compact (15-25 lines) following the chat summary format from `deconstruct`. Full analysis goes to archive.

## Worked example

### Scenario: "Major military action in Korean peninsula within 18 months"

**Threshold:** any of: (a) North Korean kinetic action across DMZ at scale beyond border incidents, (b) South Korean preemptive action, (c) US strikes on North Korean infrastructure, (d) Chinese intervention pre-empting any of these

**T-365 indicators:**
- Personnel changes: military leadership rotation patterns (any unusual concentration of hardline figures in command roles)
- Doctrine: published military exercises focusing on specific operational concepts (e.g., shift toward integrated air-ground operations)
- R&D: priorities shifting toward specific weapon classes (long-range precision, missile defense penetration)
- Diplomatic: changes in pattern of high-level meetings (sudden coordination Russia-NK, or sudden China-NK distancing)

**T-180 indicators:**
- Resource allocation: budget shifts, materiel pre-positioning visible in commercial satellite imagery
- Information operations: shifts in domestic propaganda narratives (preparation of population for sacrifice or victory)
- Leadership statements: subtle shifts from deterrence rhetoric to victory-confidence rhetoric
- Allied behavior: South Korean civil defense exercises increase, Japan deployment patterns shift

**T-90 indicators:**
- Force movements: visible deployment changes, especially concentration near borders
- Financial: capital outflow from regional currencies accelerates
- Diplomatic: North Korean diplomats recalled, embassy personnel evacuated
- Logistics: civilian ship movements through Yellow Sea / Sea of Japan altered

**T-30 indicators:**
- Communications: marked changes in NK leadership communications patterns
- Civilian movement: evacuation patterns in border areas
- Stockpiling: sudden hoarding behaviors, fuel and food
- Financial: insider-knowledge financial positions appearing in markets

**Baseline:** Each indicator has measurable normal. E.g., "personnel rotations" — track over 5-year baseline to detect anomalies. "Civilian ship movements" — track normal patterns vs alert patterns.

**Alert thresholds:**
- Yellow: 3+ T-180 indicators active simultaneously, OR 1 T-90 indicator at "deviating"
- Orange: 4+ T-180 indicators sharply deviating, OR 2+ T-90 indicators active, OR 1 T-30 indicator
- Red: any T-30 indicator at sharply deviating, OR 3+ T-90 indicators active

**Current state (May 2026 hypothetical):**
- Yellow: T-180 indicators show some elevation (1-2 active)
- Not at threshold for any alert level
- Continued monthly monitoring

### Scenario: "Major US-China financial decoupling event within 12 months"

**Threshold:** any of: (a) US sanctions on Chinese major banks, (b) Chinese capital controls on US Treasuries, (c) cross-listing major restrictions, (d) reserve currency policy shifts visible in central bank actions

**T-365 indicators:**
- Treasury holdings: Chinese reserves composition changes (slow but observable trend)
- Cross-listings: pattern of major Chinese companies delisting or relisting in alternative venues
- Bilateral talks: cadence and outcome of US-China economic dialogues
- Regulation: drafted regulations under public comment that would target specific cross-border activities

**T-180 indicators:**
- Specific company actions: BYD, Alibaba, others adjusting US listing strategies
- US sanctions packages: targeting specific institutions or sectors visible in Treasury OFAC publications
- Currency: yuan moves outside trading bands without intervention

**T-90 indicators:**
- Concrete sanctions: specific institutions sanctioned
- Capital flow restrictions: specific Chinese restrictions on specific outflows
- Counter-actions: Chinese targeted measures against US firms

**T-30 indicators:**
- Major financial firms repositioning visible in transaction data
- Insurance pricing on USD-yuan transactions changing
- Specific decoupling actions

**Alert thresholds:**
- Yellow: 4+ T-180 indicators active
- Orange: 2+ T-90 indicators active or any T-30
- Red: multiple T-30 active

## Anti-patterns

- **Topics instead of scenarios.** "Watch China" is a topic, not a watchlist entry. Without specific scenario, no specific indicators can be defined.
- **Indicators that are always active.** If an indicator has been "active" for years without any event occurring, it's not predictive. Remove or refine.
- **Indicators that are never active.** If an indicator has not been observed in years, the threshold may be set too high. Recalibrate.
- **No baseline.** Indicator "active when X above threshold Y" without knowing what normal X looks like. Indicator is uninformative.
- **Watchlist drift.** Adding scenarios faster than retiring them. Watchlist becomes overwhelming, monitoring quality drops. Discipline: 5-15 active, periodic retirement.
- **Alert fatigue.** Setting thresholds too low → constant yellow alerts → owner stops paying attention → real alerts get missed. Set thresholds for genuine signals.
- **Scenario inertia.** Keeping scenarios long after their windows have closed or they've resolved. Quarterly cleanup is mandatory.
- **Indicator clustering by domain.** All military indicators for a war scenario, all financial indicators for a financial scenario. Real preparation often shows in adjacent domains. Best indicators are sometimes cross-domain (military preparation visible first in financial repositioning by insiders).
- **Confirmation bias in indicator design.** Designing indicators that confirm what you expect. Discipline: include indicators that would activate even if your priors are wrong.

## Output template

```
─── I&W WATCHLIST ENTRY ───

Scenario ID: <unique>
Scenario: <specific event or transition with time horizon>
Threshold: <operational definition of "happening">
Created: <date>
Status: [active monitoring | yellow alert | orange alert | red alert | retired]

Indicators by horizon:

T-365:
- <Indicator>: source <where to look>, baseline <what normal looks like>, threshold <what counts as active>
- ...

T-180: <same structure>
T-90: <same structure>
T-60: <same structure>
T-30: <same structure>

Alert thresholds:
- Yellow: <conditions>
- Orange: <conditions>
- Red: <conditions>

Current state:
Active indicators: <list with dates of activation>
Alert level: <current>
Last reviewed: <date>
Next review: <date>

History:
- <Indicator activations log>
- <Alert level transitions>
```

## Integration with ОСП and infrastructure

**Watchlist is a Prisma model:** `WatchlistScenario`, `Indicator`, `IndicatorActivation` — full DB structure for tracking.

**Scheduler integration:**
- Daily 09:00 — check T-30 indicators for active alerts
- Weekly Monday 09:00 — check T-60 to T-90 for all scenarios
- Monthly 1st — check T-180, full review
- Quarterly — major watchlist cleanup and addition

**ОСП escalation flow:**
1. Indicator activates → updated in DB
2. Scenario state recomputed
3. If alert threshold crossed, ОСП auto-triggered at appropriate depth on scenario
4. Analysis result feeds back to watchlist (refines indicators, updates baseline)

**Owner notification flow:**
- Yellow: weekly digest mention
- Orange: dedicated notification with scenario state and recommendation
- Red: immediate notification with action recommendation

This skill, more than any other, makes ОСП **proactive** rather than **reactive**. Without it, ОСП waits for owner to ask. With it, ОСП raises hand when preparation patterns emerge — before crises become public knowledge.

For asymmetric investment opportunities (the Rheinmetall-before-Iran case), this is the mechanism. The major events that produce 5-10x returns for early movers always have preparation phases. Watchlist catches them.
