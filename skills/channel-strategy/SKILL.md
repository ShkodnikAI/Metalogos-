---
name: channel-strategy
description: Choosing where to promote — which channels reach the target segment, at what cost, with what fit. A perfect message in the wrong channel reaches no one. Covers channel selection, channel-message fit, budget allocation across channels, and avoiding the "be everywhere" trap.
---

# Channel Strategy — Reach the Right People Where They Are

The best message, in a channel your audience doesn't use, converts no one. Channel strategy is the discipline of choosing *where* to promote — matched to where the segment actually is, at a cost the economics support.

## Prerequisites

- `audience-segmentation` — the "where to reach" field of the segment
- `unit-economics-and-cac` — channel cost must fit the economics
- `positioning-and-messaging` — the message that channels will carry

## Core principle

> A channel is not a megaphone you point at the world — it is a specific place where a specific audience already gathers. The question is never "which channels are popular" but "where is THIS segment, and can we reach them there affordably". Being everywhere is being nowhere, expensively.

## The channel-segment fit principle

Channel selection follows directly from segmentation. The segment's "where to reach" field already names the channels. Channel strategy refines and prioritizes them.

A channel is a fit when:
- The target segment genuinely gathers there
- The channel's format suits the message and awareness level
- The cost to reach them produces an acceptable CAC
- You can actually execute in that channel

Miss any of these and the channel is wrong, no matter how popular it is.

## Channel categories

**Owned** — channels you control: your own bot, your existing user base, your content. Lowest cost, limited reach (only people already connected).

**Earned** — reach you don't pay for directly: word of mouth, referrals, community mentions, organic search. High trust, slow and hard to control.

**Paid** — channels you buy into: paid placements, ads, sponsorships. Fast, scalable, controllable — but costs accrue and stop when spend stops.

A healthy strategy usually mixes them: paid to start and scale, owned to retain and re-engage, earned compounding over time. But mix only where each channel genuinely fits the segment — not for the sake of "coverage".

## Channel-message fit

Channels differ in what message they can carry:

- **Short-format channels** — a hook and a link. Suits most-aware audiences (clear offer) or problem-aware (sharp pain hook). Can't carry education.
- **Long-format channels** — room to explain. Suits unaware/problem-aware audiences who need the problem surfaced or the solution explained.
- **Conversational channels** — two-way. Suits objection-handling, product-aware audiences.
- **Visual channels** — show, don't tell. Suits products with a visible benefit.

The awareness level (from `positioning-and-messaging`) constrains channel choice: an unaware audience needs a channel with room to educate; a short-format channel can't do that job.

## The "be everywhere" trap

The instinct to be on every channel is the most expensive mistake in channel strategy. It fails because:

- Budget spread thin across many channels means none gets enough to work
- Each channel needs its own format adaptation — many channels = much work, done shallowly
- Measurement gets muddy — hard to tell what's working
- A small operation cannot execute well on many channels at once

The discipline: **start with one or two channels that best fit the priority segment**. Prove they work. Add channels deliberately, one at a time, only when the current ones are working and there's capacity. Depth before breadth.

## The procedure

### Step 1 — List candidate channels
From the segment's "where to reach" field, plus any other plausible channels. Categorize: owned / earned / paid.

### Step 2 — Score each for fit
For each channel: does the segment really gather there? Does the format fit the message and awareness level? Can we execute it?

### Step 3 — Estimate cost and CAC per channel
Roughly, what does it cost to reach the segment in this channel, and what CAC would that imply (using forecast conversion)? A channel implying a CAC above customer value is out, however good the fit.

### Step 4 — Pick 1-2 priority channels
Choose the best-fit, best-economics channels. One or two — not five. Resist "be everywhere".

### Step 5 — Adapt the message per channel
The core message (`positioning-and-messaging`) gets format-adapted to each chosen channel — short-format gets the hook, long-format gets the fuller version.

### Step 6 — Allocate budget
Split the budget across the chosen channels. Don't spread evenly by default — weight toward the best-fit channel. Keep a portion for testing.

### Step 7 — Plan for measurement
Each channel must be measurable separately, or you can't verify which worked. Distinct links/codes per channel.

## Worked example

Channel strategy for the Fosved miniapp, Segment A "Overloaded solo operator", problem-aware.

**Candidate channels (from segment "where to reach"):**
- Small-business Telegram channels — paid placement (paid)
- Entrepreneurship newsletters — paid placement (paid)
- The existing fosved-bot user base — direct message (owned)
- Word of mouth from current users (earned)

**Fit scoring:**
- *Telegram channel placements* — segment gathers there: yes. Format: short — fine for a problem-aware hook. Execution: easy. Cost: moderate. BUT the prior campaign verification flagged this channel converting at ~0.9% — high implied CAC. Fit: questionable on economics.
- *Newsletters* — segment gathers there: yes. Format: longer — good for problem-aware, room to name the pain properly. Execution: moderate. Cost: moderate. Fit: good.
- *Owned fosved-bot users* — these are existing users; some may be in Segment A and not know about the miniapp. Format: conversational. Cost: ~zero. Fit: excellent for the people it can reach, but limited reach.
- *Word of mouth* — slow, can't be switched on. Fit: real but not a campaign lever.

**Decision:** priority channels — **owned fosved-bot user base** (near-zero cost, excellent fit, prove the message here first) + **newsletters** (good format fit for problem-aware, better economics than the Telegram channels that underperformed before). **Telegram channel placement deprioritized** — the prior verification showed bad economics there; not repeating that mistake without a reason.

**Budget:** majority to newsletters (the paid lever), small test portion held back; owned channel costs nothing but execution time.

**Measurement:** distinct link per channel so `campaign-verification` can attribute conversions.

Note: this is two channels, not five. It uses the prior campaign's verification lesson (Telegram placement underperformed) instead of repeating it. That's channel strategy learning from the archive.

## Anti-patterns

- **Be everywhere.** Spreading thin budget across many channels; none works. The signature mistake.
- **Popular over fit.** Choosing a channel because it's big, not because the segment is there.
- **Ignoring channel-message fit.** A long educational message in a short-format channel; a "buy now" in a channel full of unaware people.
- **Ignoring channel economics.** A well-fitting channel whose cost implies a CAC above customer value.
- **Even budget split by default.** Equal money to every channel regardless of fit. Weight toward the best.
- **No per-channel measurement.** One link for all channels — can't tell what worked.
- **Repeating a failed channel.** The archive shows a channel underperformed; using it again with no change or reason.
- **Owned-channel blindness.** Buying paid reach while ignoring the existing user base that costs nothing.
- **Adding channels before proving current ones.** Breadth before depth; expansion before the basics work.

## Output template

```
CHANNEL STRATEGY — <campaign>
Target segment: <segment>  |  Awareness level: <level>

CANDIDATE CHANNELS
<channel> [owned/earned/paid]
  Segment gathers here? <yes/no — evidence>
  Format fit (message + awareness): <good/poor — why>
  Estimated cost / implied CAC: <...>
  Executable for us? <yes/no>
[repeat]

ARCHIVE CHECK: <any past campaign learnings about these channels>

PRIORITY CHANNELS (1-2)
<chosen channels and why>

DEPRIORITIZED
<channels not chosen and why>

PER-CHANNEL MESSAGE ADAPTATION
<channel>: <how the core message is adapted to this channel's format>

BUDGET ALLOCATION
<channel>: <amount / %>   (test reserve: <amount>)

MEASUREMENT PLAN
<how each channel is tracked separately>
```

This populates the channels field of `MarketingCampaign`.

## Integration

- Tier 2 — loaded for campaign tasks
- Built on `audience-segmentation` ("where to reach") and `positioning-and-messaging` (the message + awareness level)
- `unit-economics-and-cac` — per-channel cost must fit the economics
- `campaign-verification` — per-channel measurement enables attribution; channel lessons feed back here
- `ab-testing-protocol` — channels may be tested against each other
