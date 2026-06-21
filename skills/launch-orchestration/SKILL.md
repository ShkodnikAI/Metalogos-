---
name: launch-orchestration
description: Coordinating a full product launch — the larger, multi-stage promotion of a new product or major release. Sequencing pre-launch, launch, and post-launch phases; coordinating with other departments; managing the launch as a campaign with its own forecast and verification. For significant launches, not routine campaigns.
---

# Launch Orchestration — Coordinating a Full Product Launch

A routine campaign promotes an existing thing. A launch introduces a new product or major release to the world — bigger, multi-stage, and crossing department lines. This skill is how a launch is orchestrated so it doesn't collapse into chaos.

## Prerequisites

- All Tier 1 marketing skills, plus `channel-strategy` and `positioning-and-messaging`
- A product/release genuinely ready (or with a known ready date)
- This is a major event, not a routine campaign — Tier 3, used deliberately

## Core principle

> A launch is a campaign with a longer arc and more moving parts — but it is still a campaign, and the discipline doesn't change: it has a forecast made before launch, and a verification after. The orchestration adds sequencing and cross-department coordination on top of that discipline; it does not replace it. A launch with no forecast is just an expensive announcement.

## When something is a "launch" vs a "campaign"

- **Campaign** — promote an existing product to a segment. One main message, one or two channels, a defined run.
- **Launch** — introduce a new product or a major release. Multiple stages over time, multiple channels, coordination with Dev/Design/QA, often a moment-in-time event.

Use launch orchestration only for genuine launches. A routine campaign doesn't need this overhead — `forecast-before-launch` + `channel-strategy` cover it.

## The three phases

A launch sequences into three phases. Each has a purpose; skipping any weakens the launch.

### Phase 1 — Pre-launch (build awareness and readiness)

Before the product is available:
- Build awareness in the target segment — the audience should know something is coming
- Prepare all materials — messaging, channel content, visuals (often via the Visual department)
- Coordinate with Dev/Design/QA — confirm the product will actually be ready and stable on the launch date
- Set up measurement — links, codes, tracking, so the launch can be verified
- Seed earned channels — give early access or information to people who might spread the word

The pre-launch failure: building hype before confirming the product is actually ready. A launch that arrives to an unstable or unfinished product burns the awareness it built.

### Phase 2 — Launch (the moment / window)

The product becomes available:
- The core message goes out across the chosen channels
- The launch moment is concentrated — a window, not a vague drift
- Monitor closely — is the product holding up under real traffic? (coordinate with QA's production monitoring)
- Be ready to respond — questions, issues, feedback arrive fast

The launch failure: launching and not watching. Real users hit the product; problems surface; nobody is monitoring.

### Phase 3 — Post-launch (sustain and learn)

After the launch window:
- Sustain promotion — the launch moment fades; ongoing campaigns carry it forward
- Gather actual data — conversions, CAC, segment response, product feedback
- Verify the launch — forecast vs actual, exactly like a campaign (`campaign-verification`)
- Feed lessons back — to marketing skills, and to Dev/Design/QA about the product itself

The post-launch failure: treating the launch moment as the finish line. The launch is the start of the product's life, not the end of the work.

## Cross-department coordination

A launch is where the marketing department most depends on others. Yana orchestrates; marketing's job is to surface what it needs and when.

- **Dev/Design/QA** — the product must be genuinely ready and stable for the launch date. Marketing cannot build awareness for a date the product won't make. Confirm readiness *before* committing to a public date.
- **Visual** — launch materials (visuals, infographics, announcement cards) are requested from the Visual department.
- **QA production monitoring** — during the launch window, QA's daily production monitor is the early-warning system for the product breaking under real traffic.
- **Finance** — the launch budget and the CAC/payback forecast are shared with Finance for cash-flow planning.
- **Expert** — if the launch involves partnerships, press, or external relationships, Expert coordinates those.

The orchestration discipline: identify every dependency, confirm each *before* the launch date is publicly committed, and never announce a date the product side hasn't confirmed.

## The launch is still forecast and verified

The launch does not escape the department's core discipline:

- **Forecast before launch** — predicted reach, conversion, CAC, for the launch as a whole. Made *before* the launch, locked in (`forecast-before-launch`).
- **Verify after** — actual vs forecast, miss diagnosis, lessons (`campaign-verification`).

A launch is recorded as a `MarketingCampaign` with `campaignType: launch`. It gets the same forecast fields and the same verification. The multi-phase structure is additional, not a substitute.

## The procedure

### Step 1 — Confirm product readiness
Before anything else: will the product genuinely be ready and stable? Get this confirmed by Dev/Design/QA. No public date until this is solid.

### Step 2 — Forecast the launch
Reach, conversion, CAC for the whole launch. Locked before pre-launch begins.

### Step 3 — Plan the three phases
Pre-launch (what, when, which channels), launch (the window, the channels, the message), post-launch (sustaining campaigns, verification date).

### Step 4 — Map and confirm dependencies
List every cross-department dependency. Confirm each. Identify the critical ones — the ones that, if late, move the launch date.

### Step 5 — Set up measurement
Tracking in place before pre-launch starts, so the whole launch is verifiable.

### Step 6 — Execute pre-launch
Build awareness, prepare materials, seed earned channels.

### Step 7 — Execute the launch window
Message out, monitor closely (with QA), respond fast.

### Step 8 — Execute post-launch
Sustain with ongoing campaigns, gather data.

### Step 9 — Verify
Forecast vs actual. Diagnose. Lessons to marketing skills and to the product departments.

## Worked example (abbreviated)

Launch: a major new release of the Fosved miniapp.

- **Readiness check:** Dev/Design/QA confirm the release is feature-complete and QA-green for a target date three weeks out. Only then is the date set.
- **Forecast:** the launch as a whole — reach ~15,000 across owned + newsletter channels, conversion 2-3%, CAC ~120. Locked.
- **Pre-launch (weeks 1-2):** awareness content to the existing fosved-bot user base ("something new is coming"); launch visuals requested from the Visual department; newsletter placements booked; tracking links created.
- **Launch window (a defined few days):** announcement to the owned user base + newsletters go live; QA's production monitor watched closely for the release holding up under real usage; questions answered fast.
- **Post-launch (weeks after):** ongoing campaigns carry promotion forward; actual conversion and CAC gathered.
- **Verification:** forecast vs actual, miss diagnosis, lessons — to marketing skills, and product feedback to Dev/Design/QA.

The launch is one `MarketingCampaign` record, type `launch`, with the three-phase structure layered on the standard forecast-and-verify discipline.

## Anti-patterns

- **Launch with no forecast.** A big announcement nobody predicted the outcome of. Unverifiable, unlearnable.
- **Hype before readiness.** Building awareness for a date the product won't make. The awareness is wasted, trust is spent.
- **Announcing an unconfirmed date.** Committing publicly to a date the product departments haven't confirmed.
- **No pre-launch.** Launching cold, with no awareness built. The launch moment lands on an unprepared audience.
- **Launch and look away.** Not monitoring during the window. The product breaks under real traffic, unwatched.
- **Launch as finish line.** Treating the launch moment as the end. No post-launch sustaining, no verification.
- **Skipping cross-department confirmation.** Assuming the product, the visuals, the dependencies will be ready — without confirming.
- **Launch-as-campaign with no extra structure.** Treating a genuine multi-stage launch as a one-shot campaign — under-coordinated.
- **Campaign-as-launch.** Wrapping routine promotion in heavy launch orchestration it doesn't need.

## Output template

```
LAUNCH ORCHESTRATION — <product / release>

PRODUCT READINESS
Confirmed ready by Dev/Design/QA? <yes — date / not yet>
Target launch date: <date>  (set only after readiness confirmed)

LAUNCH FORECAST (locked before pre-launch)
Reach: <range>  |  Conversion: <range>  |  CAC: <forecast>
Recorded as MarketingCampaign type=launch.

PHASE PLAN
Pre-launch (<dates>): <awareness activities, materials, channels, seeding>
Launch window (<dates>): <channels, message, monitoring>
Post-launch (<dates>): <sustaining campaigns, data gathering, verification date>

CROSS-DEPARTMENT DEPENDENCIES
- Dev/Design/QA: <what's needed, confirmed?>
- Visual: <materials needed>
- QA monitoring: <production watch during window>
- Finance: <budget + CAC/payback shared>
- Expert: <if partnerships/press involved>
Critical-path dependencies: <the ones that move the date if late>

MEASUREMENT
<tracking set up before pre-launch>

VERIFICATION DATE: <when forecast vs actual is checked>
```

## Integration

- Tier 3 — used for genuine launches, not routine campaigns
- Built on all Tier 1 skills + `channel-strategy` + `positioning-and-messaging`
- The launch is a `MarketingCampaign` (type `launch`) — `forecast-before-launch` and `campaign-verification` still apply
- Heavy coordination with Dev/Design/QA, Visual, Finance, Expert — orchestrated via Yana
- `ab-testing-protocol` may be used within launch campaigns
