---
name: positioning-and-messaging
description: Crafting positioning (how a product is framed in the customer's mind) and messaging (the actual words). Built on the audience's awareness level and the differentiation gap. Turns "what the product does" into "why this specific person should care". The core creative skill of the department.
---

# Positioning and Messaging — Why This Person Should Care

A product description says what the product does. Positioning says why it matters, to whom, against what. Messaging is the actual words that carry it. This is where research becomes persuasion — honestly.

## Prerequisites

- `audience-segmentation` — the target segment is defined
- `competitive-analysis` — the differentiation gap is identified
- `honest-claims-discipline` — every claim produced here gets checked against it

## Core principle

> Positioning is not what you say about the product — it is the slot the product occupies in the customer's mind, relative to their alternatives. You don't describe the product; you answer the customer's real question: "why should I, specifically, in my situation, choose this over what I'm doing now?" Get the slot right and the words follow. Get it wrong and no amount of clever copy saves it.

## Positioning before messaging

Order matters. Positioning is the strategic decision; messaging is its expression. Writing copy before positioning is decided produces clever words pointing nowhere.

**Positioning answers:**
- For whom is this? (the segment)
- What category is it in / compared against? (the frame of reference)
- What's the key benefit? (the one thing that matters most to this segment)
- Why is that believable? (the reason to believe — ties to the differentiation gap)

**Messaging answers:**
- What exact words carry the positioning?
- What's the headline, the supporting points, the call to action?

## Awareness levels — the most important input

Eugene Schwartz's five levels of audience awareness determine *what the message can even say*. Messaging the wrong level fails completely.

1. **Unaware** — doesn't know they have the problem. Message must start by surfacing the problem, gently. Selling the product directly fails — they don't know why they'd need it.
2. **Problem-aware** — feels the pain, doesn't know solutions exist. Message names the pain sharply and reveals that a solution category exists.
3. **Solution-aware** — knows solutions exist, doesn't know yours. Message positions your product within the category and against alternatives.
4. **Product-aware** — knows your product, not convinced. Message handles objections, proves the claims, differentiates.
5. **Most aware** — ready, just needs the trigger. Message is a clear offer and call to action; don't over-explain.

A message written for the most-aware ("Sign up now!") thrown at an unaware audience is noise. A long problem-education message thrown at a most-aware audience bores them out of converting. **Match the message to the level.**

The awareness level comes from `audience-segmentation` — it's one of the segmenting dimensions.

## Lead with the job, not the feature

People don't buy features. They buy a better version of their situation — the job-to-be-done.

- ✗ Feature: "Real-time data synchronization across modules."
- ✓ Job: "Stop re-checking five places to know where your business stands."

The feature is *how*; the job is *why they care*. Messaging leads with the why. The how can follow as the reason-to-believe.

This connects to the segment's pain point — the message names the pain in the customer's own words, then shows the job done.

## Speak the customer's language

The words come from the audience, not from inside the company. Internal language ("consolidated analytics dashboard") rarely matches how customers describe their own problem ("I never know where things stand").

- Use the phrases real customers used in interviews and support conversations (`market-research-methods` gathers these)
- Drop jargon the segment doesn't use
- Match their register — formal or casual as *they* are, not as the company is

The empathy of the department's psychotype shows here: the message is written from inside the customer's head, in their words, about their situation.

## The procedure

### Step 1 — Confirm the inputs
Segment defined? Differentiation gap identified? Awareness level known? If not, go back — messaging without these is guesswork.

### Step 2 — Write the positioning statement
A compact statement: *For [segment], who [situation/pain], [product] is a [category] that [key benefit], unlike [alternative], because [reason to believe].*

### Step 3 — Pick the message angle for the awareness level
Unaware → surface the problem. Problem-aware → name the pain + reveal solutions exist. Solution-aware → position vs alternatives. Product-aware → handle objections. Most-aware → clear offer.

### Step 4 — Draft the core message
Headline (the hook — leads with the job/pain), supporting points (2-3, the reasons to believe), call to action (one clear next step). Lead with why, support with how.

### Step 5 — Draft variants for testing
The experimenter's habit: don't write one message, write several angles. Different headlines, different pain framings. These become A/B variants (`ab-testing-protocol`).

### Step 6 — Run the claims check
Every claim through `honest-claims-discipline`. Superlatives out, specifics in, no manipulation.

## Worked example

Product: Fosved miniapp. Segment A: "Overloaded solo operator", awareness level: **problem-aware** (they feel the chaos, don't know a dedicated tool exists).

**Positioning statement:**
"For solo business operators drowning in scattered information, the Fosved miniapp is a single business view that pulls everything into one place — unlike the spreadsheet patchwork they use now, because it updates itself instead of being maintained by hand."

**Awareness-matched angle:** problem-aware → name the pain sharply, then reveal the solution category exists. Do NOT jump straight to "sign up" (they're not most-aware), do NOT educate them that they have a problem (they already feel it).

**Core message:**
- Headline (leads with the job/pain): "Stop guessing where your business stands."
- Supporting points (reasons to believe): "Your notes, numbers, and updates — pulled into one view." / "It updates itself, so you stop maintaining a fragile spreadsheet." / "Built for one person running everything, not for teams."
- Call to action: "See your business in one view — open the Fosved miniapp."

**Variants for testing:**
- Angle A (pain-led): "Stop guessing where your business stands."
- Angle B (time-led): "Quit re-checking five places to know one thing."
- Angle C (control-led): "Run your whole business from one view."

These three go to `ab-testing-protocol`. Note all three are honest (no superlatives), all match problem-aware, all lead with the job — they differ only in which facet of the pain they hook.

## Anti-patterns

- **Messaging before positioning.** Clever copy with no strategic slot behind it.
- **Wrong awareness level.** "Sign up now!" to an unaware audience; a problem-education essay to a ready buyer.
- **Feature-led messaging.** Listing what the product does; never saying why the customer cares.
- **Company language.** "Consolidated analytics dashboard" when customers say "I never know where things stand."
- **Positioning for everyone.** A message diluted to offend no segment, landing with none. (Root cause: skipped `audience-segmentation`.)
- **Superlative-stuffed copy.** "The best, most powerful, revolutionary..." — fails `honest-claims-discipline` and persuades no one.
- **One message, no variants.** No A/B options; no way to learn which angle works.
- **Ignoring the differentiation gap.** Positioning that doesn't connect to a real, defensible difference.
- **Borrowed positioning.** Copying a competitor's slot — now you're the second-best occupant of their position.
- **Clever over clear.** Wordplay that obscures the message. Clarity converts; cleverness usually doesn't.

## Output template

```
POSITIONING & MESSAGING — <product>

Target segment: <segment name / id>
Awareness level: unaware | problem-aware | solution-aware | product-aware | most-aware
Differentiation gap: <from competitive-analysis>

POSITIONING STATEMENT
For <segment>, who <situation/pain>, <product> is a <category> that
<key benefit>, unlike <alternative>, because <reason to believe>.

MESSAGE ANGLE (matched to awareness level)
<the angle this awareness level requires>

CORE MESSAGE
Headline: <leads with job/pain>
Supporting points:
- <reason to believe 1>
- <reason to believe 2>
- <reason to believe 3>
Call to action: <one clear next step>

VARIANTS FOR A/B TESTING
- Angle A: <headline / framing>
- Angle B: <headline / framing>
- Angle C: <headline / framing>

CLAIMS CHECK: <passed honest-claims-discipline — yes/no>
```

This populates the positioning, coreMessage, and messageVariants fields of `MarketingCampaign`.

## Integration

- Tier 2 — loaded for campaign / creative tasks
- Built on `audience-segmentation` (segment + awareness) and `competitive-analysis` (gap)
- Output checked by `honest-claims-discipline` before launch
- Variants feed `ab-testing-protocol`
- `market-research-methods` supplies the customer's actual language
