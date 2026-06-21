---
name: bullshit-detection
description: Catalogs and identifies manipulation techniques common in technical presentations — cherry-picked demos, axis manipulation in graphs, weasel words, premature claims, jargon obscuring lack of substance, false precision, citation laundering, the appeal to authority. Provides specific recognition patterns and counter-questions. The disciplined defense against being misled by sophisticated speakers in unfamiliar domains.
---

# Bullshit Detection — Recognition Patterns For Technical Manipulation

The expert speaker has had years to optimize their pitch. They know which questions get asked, which deflections work, which framings sell. The owner has hours of preparation. Without a catalog of manipulation patterns, the asymmetry is decisive — manipulation works because the target doesn't recognize it.

This skill is the catalog. Not generic "be skeptical" advice but specific named patterns with recognition signals and counter-responses. Some patterns are conscious manipulation; others are unconscious habits of overoptimistic speakers. Recognition matters either way.

## Prerequisites

- `rapid-domain-immersion` providing baseline domain knowledge
- `project-deconstruction` providing the project's actual shape
- Awareness that good speakers may use these techniques without conscious deception — the issue is whether information is accurately conveyed, not whether speaker is "lying"

## Core principle

> Manipulation in technical presentations follows recognizable patterns. Cataloging the patterns and their countermeasures inverts the asymmetry — the prepared listener can identify manipulation faster than the speaker can deploy it. The discipline isn't suspicion; it's recognition. Speakers who deploy multiple manipulation techniques are signaling something about their substance.

## The catalog of patterns

### Pattern 1 — Cherry-picked demos

**What it looks like:**
Demo shows specific scenario where technology works perfectly. Other scenarios not mentioned or shown in passing.

**Recognition:**
- Demo conditions described in detail (this seed, this lighting, this dataset)
- "We chose this example because it illustrates" (revealing it's chosen, not representative)
- No comparison demos (only success shown)
- Single-take demonstration (not stress test)

**Counter:**
"What's the scenario where this approach works least well? Can you show that?"
"What's the variance — across N attempts, how often does it produce this quality?"

### Pattern 2 — Axis manipulation in graphs

**What it looks like:**
Visual claims dramatic improvement that, examined, is small or contextless.

**Recognition:**
- Y-axis doesn't start at zero (small differences look big)
- Logarithmic scale unstated (changes meaning of "doubled")
- Time scale compressed or expanded selectively
- Comparison group missing (improvement vs what?)
- Cumulative graphs showing growth that flattens recently

**Counter:**
"What's the absolute number, not the percentage?"
"What's the comparison baseline — vs what alternative?"
"Show me the same data on linear scale starting from zero"

### Pattern 3 — Weasel words

**What it looks like:**
Claims technically true but practically meaningless.

**Specific weasel words and what they hide:**
- "Up to" — typically far below this
- "Approximately" — wider than typical use
- "Industry-leading" — within a definition that excludes major competitors
- "Proven technology" — proven in some context, not necessarily the relevant one
- "Patented technology" — patent doesn't mean it works or matters
- "Revolutionary" — empty marker
- "First in [narrow category]" — narrow category invented to enable claim
- "Validated by [authority]" — validated for some purpose, not necessarily the implied one

**Counter:**
"What's the typical [metric] not the maximum?"
"Industry-leading among which competitors specifically?"
"Proven in what specific deployment?"
"Validated for what use case? The validation document please."

### Pattern 4 — Premature claims

**What it looks like:**
Claims about commercial readiness, performance, or timeline that exceed actual demonstration.

**Recognition:**
- Production language for prototype-stage work
- Future tense slipping into present ("will deliver" → "delivers")
- Claimed customers who turn out to be pilots, LOIs, or mentioned-as-interested
- Performance numbers from optimal lab conditions presented as deployment numbers
- Timelines that compound multiple uncertain steps as if certain

**Counter:**
"How many production deployments today, generating revenue?"
"Is this number from lab conditions or from deployment?"
"The customers you mention — what's the contract value with each?"
"Walk me through the dependency chain on that timeline"

### Pattern 5 — Jargon as obscuration

**What it looks like:**
Technical language used to make simple ideas sound profound, or to prevent questioning by suggesting expertise the speaker doesn't have.

**Recognition:**
- Words used in non-standard ways without explanation
- Multiple jargon terms strung together without connection
- Speaker can't define their own jargon when asked
- Different jargon for same concept across slides
- Terms borrowed from sophisticated fields (quantum, AI, neural) for non-relevant applications

**Counter:**
"Can you explain that without [specific jargon term]?"
"What's the simple version of that?"
"How does this relate to [established concept in field]?"

### Pattern 6 — False precision

**What it looks like:**
Numbers presented with precision exceeding what's actually known or measurable.

**Recognition:**
- 3+ significant figures on estimates that should be order-of-magnitude
- Specific percentages from small samples (10x improvement based on n=3)
- Forecast numbers presented as known facts
- Unit conversions creating false precision (84.7% from 5/6 ≈ 83%)

**Counter:**
"What's the confidence interval on that number?"
"What's the sample size?"
"How was that measured — what's the methodology?"

### Pattern 7 — Citation laundering

**What it looks like:**
Claims attributed to authoritative-sounding sources that don't actually support the claim.

**Recognition:**
- "Published in Nature" — but doesn't mention what the paper actually showed
- "MIT/Stanford/Harvard" — affiliation of one researcher implies institutional endorsement
- "FDA approved" — for what specifically, in what year
- "Industry standard" — set by which body
- Specific number from "research" without naming research

**Counter:**
"Who's the paper author and what did the paper specifically conclude?"
"Was the institution involved or just a researcher there?"
"Approved for which indication, when, for which population?"
"Which standards body specifically?"

### Pattern 8 — Appeal to authority

**What it looks like:**
Substituting authority of person/organization for technical merit.

**Recognition:**
- Heavy emphasis on team credentials, less on technical specifics
- Names of famous figures (advisors, investors, customers) without their substantive role
- Awards and recognitions (often paid or marketing)
- Repeated reference to specific famous person who endorsed
- Distinguished employees from other contexts ("ex-Google engineers" without specifying their relevant work)

**Counter:**
"What's the specific technical contribution of [advisor]?"
"What does [famous customer] use this for in production?"
"What's the recognition criteria for [award]?"

### Pattern 9 — Strategic ambiguity

**What it looks like:**
Claims phrased to seem strong but technically weak when examined.

**Recognition:**
- "Working with" (not "deployed for")
- "Partnership with" (often LOIs, not contracts)
- "Featured in" (paid placement?)
- "Used by" (one user? Or many? In what role?)
- "Trusted by" (vague)
- Active voice covering passive reality ("we developed" when they licensed)

**Counter:**
"What does 'partnership' mean specifically — paid contract?"
"How many users, what's the deployment pattern?"
"What did you develop yourselves vs license/buy?"

### Pattern 10 — Question deflection

**What it looks like:**
When asked specific technical question, response answers different question.

**Recognition:**
- "Great question — what's really important is..." (pivoting)
- Returning to prepared talking points instead of answering
- Long preamble before any actual answer
- Rephrasing question into easier version, then answering rephrased
- "We covered that earlier" (without actually having)
- Personal anecdote replacing technical answer

**Counter:**
Repeat the question. Word-for-word.
"That's interesting, but specifically what I asked was [restate]"
"Let me ask differently: [rephrased same question]"

### Pattern 11 — False dichotomy

**What it looks like:**
Presenting a choice as binary when alternatives exist.

**Recognition:**
- "It's our approach or you're stuck with current technology"
- "Either you partner with us now or you miss the wave"
- "You can spend 10x the cost on incumbents or trust us"
- Erasing the "do nothing" option

**Counter:**
"What about [specific third option]?"
"What if the right answer is to wait?"
"What about competitor X who claims similar benefits?"

### Pattern 12 — Future evidence promise

**What it looks like:**
Important claims defended by reference to evidence not yet available.

**Recognition:**
- "Our trial results coming next quarter will show..."
- "When we publish, you'll see..."
- "The next demonstration will prove..."
- "We can't show that yet but [strong claim about it]"

**Counter:**
"What can you show me today that demonstrates this?"
"Can I see a partial result, even if preliminary?"
"What's the delay in providing this?"

## The procedure

### Step 1 — Review project materials with manipulation lens

After project deconstruction, re-read materials specifically asking: where might manipulation patterns be present?

### Step 2 — Tag identified patterns

For each pattern found, note:
- Which pattern category
- Specific instance (quote or description)
- What it might be hiding
- Whether multiple patterns cluster

### Step 3 — Cluster analysis

Single occurrences of a pattern can be honest oversight. **Multiple patterns clustering** is a signal — speakers using 3+ techniques systematically reveal something about the project's substance.

### Step 4 — Generate counter-questions

For high-priority patterns identified, prepare counter-questions. These integrate into the combat questions designed in Phase 4.

### Step 5 — Prepare red flags list for owner

Distill into a reference card the owner can keep in mind during meeting. Format: "If you hear X, listen for Y, that's a sign of Z."

### Step 6 — Output bullshit-detection report

Structured catalog of identified patterns plus owner-facing red flags reference.

## Worked example — fictional fusion startup pitch

Reviewing FusionCorp materials with manipulation lens.

**Pattern 7 (citation laundering):**
"As featured in Nature." Investigation: Article in Nature was a brief mention in a sector overview, not paper about their technology. Counter: "The Nature article — what specifically did it say about your technology?"

**Pattern 4 (premature claims):**
"Production-ready compact tokamak." Reality: prototype demonstrating subscale plasma physics. Counter: "Walk me through your largest deployment today, generating real customer value."

**Pattern 6 (false precision):**
"$2.97 billion capital cost per gigawatt." Three sig figs on a forecast 5+ years out, no demonstration data behind. Counter: "What's the range or confidence interval on that figure? What's it based on?"

**Pattern 8 (appeal to authority):**
Heavy emphasis on advisor list including Nobel laureate. Investigation: Advisor's expertise is plasma physics in different approach (stellarators, not tokamaks). Counter: "What's [advisor]'s specific contribution to your tokamak design?"

**Pattern 12 (future evidence):**
"Our SPARC-equivalent demonstration in late 2027 will validate Q=10." Major claim deferred to future result. Counter: "Today, what's your plasma performance compared to the JET record?"

**Cluster signal:** Multiple patterns deployed (5+ in one pitch deck) — this isn't random oversight, it's systematic. Suggests team doesn't have substance to fill the space, or chooses style over substance, or actively misleading. Either way, treat all claims with elevated skepticism.

**Red flags for owner:**
- Watch for: production-ready language for what's clearly prototype work
- Watch for: numbers presented with high precision but without confidence range
- Watch for: deflection to advisor names when pressed on specifics
- Watch for: invocation of "Nature published our work" without paper specifics

## Anti-patterns

- **Treating all manipulation techniques equally.** Some are common honest habits (false precision is everywhere). Some are deliberate (cherry-picked demos in formal investor pitches usually deliberate).
- **Over-detecting.** Suspicion of everything is paralysis. Single weasel word is normal speech. Pattern clusters matter.
- **Confusing slick presentation with manipulation.** Polished delivery isn't dishonest. Substance matters more than style.
- **Detection without action.** Identifying patterns without using them in combat questions is incomplete.
- **Public confrontation in meeting.** Recognition is for the owner's interpretation, not for "calling out" the speaker. Adversarial meeting destroys information flow. Use detection to ask better questions, not to embarrass.
- **Generic "be skeptical" advice.** Specific patterns with specific countermeasures are what work.

## Output template

```
─── BULLSHIT DETECTION ───

Project: <identifier>
Materials reviewed: <list>

PATTERNS IDENTIFIED:

Pattern <#>: <name>
Instance: "<specific quote or description from materials>"
What it might hide: <hypothesis>
Counter-question: "<question for combat list>"

[Multiple patterns]

CLUSTER ANALYSIS:
- Total patterns identified: <count>
- Concentration of patterns: <high | medium | low>
- Cluster signal: <interpretation>

RED FLAGS REFERENCE FOR OWNER:
- If you hear "<phrase>", listen for: <what to verify>, that's a sign of: <pattern>
- [list of 5-10 red flags]

INTEGRATION WITH COMBAT QUESTIONS:
Questions added to combat list based on detection: <list>
```

## Integration with Expert protocol

This is a **Tier 1 skill that operates parallel to** the main flow rather than as a sequential phase. Detection happens during materials review (Phase 2 deconstruction) but produces output integrated into combat questions (Phase 4).

Output stored in ExpertBriefing record under `bullshitDetection` field. Patterns identified become reference material for future briefings (the catalog improves through accumulation of recognized patterns).

The ultimate test: in post-meeting debrief, did the speakers actually deploy the predicted manipulation techniques? Each correct prediction strengthens the catalog. Each missed manipulation reveals new pattern to add.
