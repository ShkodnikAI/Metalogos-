---
name: strategic-maneuvers
description: Catalog of strategic maneuvers from Sun Tzu and modern strategic theory, applied to the subject of analysis. Used after forces-and-potentials computation to answer the question — given the current differential, what maneuvers are available to the subject (especially when subject has lower potential than opposing force)? Generates 2-4 candidate maneuvers with feasibility assessment, expected effect on the vector field, and triggers that would activate them.
---

# Strategic Maneuvers — When Smaller Force Wins

The mistake of pure-physics analysis is to assume that the larger potential always wins. The mistake of pure-game-theory analysis is to assume that smart maneuvers can compensate for any potential gap. The truth is between: maneuvers operate within a potential field — they cannot reverse a 10× gap, but they can convert a losing 1.5× gap into a stalemate, or a stalemate into a win, or convert direct conflict into indirect victory.

This skill is the catalog of maneuvers. Sun Tzu wrote 2500 years ago and his principles remain the operational core, augmented by modern strategic theory (Boyd's OODA, Liddell Hart's indirect approach, Marshall's Net Assessment). They're not metaphors — they're operational categories.

## Prerequisites

- `awareness-frame` and `forces-and-potentials` completed
- Subject identified, including its current differential D and force composition
- Understanding that this skill produces **options**, not single recommendations

## Core principle

> The position you take before fighting decides whether fighting is necessary. Maneuvers shape position. Sometimes the maneuver eliminates the need for force; sometimes it concentrates force at the decisive point; sometimes it borrows third-party force to substitute for one's own. All three are superior to direct contest.

The error mode is to think tactically — "what action should subject take next?" The right mode is to think positionally — "what configuration in the field of forces does subject want, and what maneuver moves it there?"

## The six core maneuvers

### Maneuver 1 — Shape (形, xíng)

**Principle:** "Be such that the enemy must come to you on terms favorable to you."

The subject does not engage where the opponent is strong; the subject changes its position so that any engagement happens on terrain favorable to itself.

**For Belarus example:** Belarus cannot militarily resist Russia. But Belarus can position itself as transit critical to multiple parties (Russian energy, Chinese exports, EU goods) — making any aggressive Russian move costly to Russia's other relationships.

**Applicability:** when subject has lower direct potential but has options to reposition. Requires time. Doesn't work in acute phase.

**Feasibility check:** does subject have time? does subject have alternative positions available? does opponent care about its other relationships?

### Maneuver 2 — Deception (詭道, guǐdào)

**Principle:** "Show weakness where strong, show strength where weak."

The subject manages opponent's perception of subject's potential. Underestimated subject has more freedom to maneuver. Overestimated subject deters opponent from moves opponent would otherwise make.

**For Belarus example:** Lukashenko regularly oscillates publicly between pro-Russia and conditional independence statements, keeping Russia uncertain about the cost of any escalation.

**Applicability:** when opponent's actions depend on opponent's reading of subject. Less applicable when opponent has accurate intelligence.

**Feasibility check:** does subject control information channels to opponent? can subject maintain consistent deception or will it be detected?

### Maneuver 3 — Empty/Full (虚実, xūshí)

**Principle:** "Attack the empty, avoid the full."

Forces have strong points and weak points. Even a weaker subject can prevail by concentrating its limited resources at the opponent's weakness while avoiding the opponent's strength.

**For investment example:** The largest defense contractors are "full" — well-defended, priced-in, hard to find edge. A smaller specialty supplier benefiting from same trend is "empty" — under-priced, less analyst coverage, more upside if the trend materializes. Same vector, different entry point.

**For Belarus example:** Russia is full militarily and politically. Russia is empty in technology, in international financial standing, in young demographic energy. Belarusian moves into IT services, into alternative payment infrastructure, into educational positioning — these are empty-side plays.

**Applicability:** universal — almost always there are stronger and weaker points. The discipline is identifying which is which.

**Feasibility check:** can subject identify opponent's actual weak point (vs claimed weak point)? does subject have resources to act on the weak point even if minimal?

### Maneuver 4 — Shi (勢, momentum)

**Principle:** "Use the enemy's momentum, the terrain's momentum, time's momentum, history's momentum — don't manufacture force from nothing when you can ride a wave that already exists."

Subject identifies an existing trend or force in the field that aligns with subject's interest, and positions itself to benefit from that trend without having to create it.

**For Belarus example:** Belarus does not need to create demand for transit services bypassing Russia — that demand exists from Chinese exporters and EU buyers wanting alternative routes. Belarus rides this momentum.

**For technology example:** A small company does not need to create AI demand — it exists. The company positions itself as critical infrastructure to AI applications and rides the wave.

**Applicability:** when relevant momentum exists (it usually does — search hard).

**Feasibility check:** is the momentum real and sustained, or hype? is subject early enough in the wave to benefit, or will subject arrive after the wave has been exploited?

### Maneuver 5 — Third-Force Leverage

**Principle:** "Borrow your enemy's enemy."

Subject identifies a third party with interests partially aligned with subject's interest and aligns with that third party — letting the third party's potential add to subject's effective potential.

**For Belarus example:** Belarus aligning more with China gives Belarus access to Chinese economic potential without Belarus having to develop equivalent indigenous resources. China gains Belarusian transit; Belarus gains a counterweight to Russia.

**For investment example:** The Iran war analysis identified Saudi/UAE/Israel as the actual triangle that initiated US action against Iran (H3 hypothesis). This is third-force analysis at the geopolitical level.

**Applicability:** universal — almost always there are third parties whose interests are partially aligned. Hardest part is identifying them and constructing the alignment without making the third party suspicious.

**Feasibility check:** is the third party reliable? does third-party alignment increase or decrease subject's freedom of action?

### Maneuver 6 — No-Fight Victory (不戦而屈人之兵)

**Principle:** "The supreme art of war is to subdue the enemy without fighting."

Subject achieves objective without direct contest with opponent. This requires changing the situation such that opponent's optimal action shifts to one that aligns with subject's objective — not through force but through structure.

**For Belarus example:** Rather than confronting Russian pressure on debt, Belarus could engineer its debt structure such that Russian pressure on debt becomes self-defeating (e.g., debt held by Chinese creditors who would not tolerate Russian default-pressure). The structure substitutes for direct fight.

**For investment example:** Rather than betting that company X will outperform company Y (direct contest), invest in the supplier that benefits whichever wins — the no-fight position.

**Applicability:** highest-level maneuver, rarest to find. When found, dominant.

**Feasibility check:** can the situation actually be restructured? does subject have the leverage to engineer the restructuring? does opponent see the restructure coming and counter it?

## The procedure

### Step 1 — Receive forces & potentials output

Take the differential D and force composition from `forces-and-potentials` skill output. Note especially:
- Subject's potential vs opposing potential (gap in raw multiplication)
- Vectors of each force (direction)
- Time horizon characteristics

### Step 2 — Test each maneuver for applicability

For each of 6 maneuvers, ask: is this maneuver applicable given the field of forces and time available?

Use feasibility checks above. Honest answer is sometimes "no for this maneuver in this situation." Better to eliminate than to force-fit.

### Step 3 — Generate 2-4 candidate maneuvers

Of the applicable maneuvers, generate 2-4 specific candidate maneuvers for the subject. Specific = "concrete action with concrete target" not "Belarus should pursue diplomatic flexibility." Bad: "use deception." Good: "Belarus signals openness to limited NATO partnership exercises during periods of reduced Russian pressure to extract concessions, while reaffirming CSTO during periods of Russian focus elsewhere."

### Step 4 — For each candidate, assess

Four properties:

**Direction-shift:** how would this maneuver change the vector field if successfully executed? Quantify if possible: D would shift from current value to projected value.

**Cost:** what does the subject expend (resources, optionality, reputation) to execute? Maneuvers are not free.

**Risk:** what are failure modes? Sun Tzu maneuvers can fail badly — deception detected, third force defects, momentum reverses. Honest assessment.

**Trigger:** what conditions would activate this maneuver? Maneuvers are not always-on; they need triggers (events, opportunity windows, opponent moves).

### Step 5 — Recommend with caveats

Recommend the candidates in order of risk-adjusted expected effect. Caveats:
- "This maneuver is available **if** subject can secure third-party support **before** opponent acts."
- "This maneuver requires sustained execution over months — single window isn't enough."

Do not present a single "right answer" — strategic maneuvers are options the subject's decision-makers select among, not Pavlovian responses.

## Worked example

Continuing Belarus example from `forces-and-potentials`:

D = 0.81 (decisively Russian-favorable). Subject is Belarus. Subject has lower potential by ~10x.

**Maneuver candidates:**

**Candidate 1 — Shape via transit positioning (Maneuver 1).**
Direction-shift: makes Russian aggressive moves costly via Russia's other relationships (China, EU intermediaries). D might shift from 0.81 to 0.65-0.70.
Cost: positioning takes 2-3 years; some short-term concessions to Russia required to buy time.
Risk: positioning may be detected and Russia accelerates compression to lock in Belarus before positioning matures.
Trigger: post-2026 elections globally that change EU/China posture toward Russia.

**Candidate 2 — Third-Force via China alignment (Maneuver 5).**
Direction-shift: Chinese resources backstop Belarusian elite, raising Belarus effective potential. D might shift to 0.68-0.74.
Cost: requires substantive (not symbolic) Chinese commitment; Belarus must accept Chinese influence as substitute for Russian.
Risk: China may lose interest in Belarus if Russia-China relationship priorities shift; Belarus ends up with no patron.
Trigger: Russian war outcome materially worsens, opening Chinese opportunity to expand Eurasian footprint.

**Candidate 3 — Empty/Full via tech corridor (Maneuver 3).**
Direction-shift: Belarus becomes tech-services provider that Russia also depends on, creating asymmetric dependence in unexpected direction. D minimal shift directly but creates leverage points.
Cost: requires sustained tech sector development; long horizon.
Risk: tech sector may emigrate (already happening); unclear if Russian dependence on Belarusian tech is real or hypothetical.
Trigger: Russian sanctions on Western tech force Russia to seek replacements — Belarus could position then.

**No-Fight Victory (Maneuver 6) — not currently applicable.**
The structure of the situation (Russia's preference for direct compression, no obvious restructuring path) doesn't offer a clean no-fight option. Marked as "watch — may become available if Russian war outcome creates opening."

**Recommendation:**
Combination of Candidate 2 + Candidate 1, sequenced. China alignment provides immediate backstop; transit positioning runs in parallel for medium term. Candidate 3 (tech corridor) is opportunistic — activate if trigger occurs.

## Anti-patterns

- **Confusing maneuver with tactic.** Maneuver = positional configuration in the field. Tactic = specific action. "Sign deal with China next Tuesday" is tactic. "Position Belarus as China's preferred Eurasian partner over 18 months" is maneuver.
- **Treating maneuvers as guarantees.** They shift probabilities, not certainties. A good maneuver well-executed against a smart opponent may still fail.
- **Missing feasibility check.** Recommending Maneuver 1 (Shape) when subject has no time, or Maneuver 5 (Third-Force) when no third party has aligned interests. Each maneuver has preconditions; ignore them and the maneuver is fantasy.
- **Single-maneuver thinking.** Real strategic situations call for combination — primary maneuver with backup contingencies. Generating 2-4 candidates, not one, is part of the discipline.
- **Maneuvers without triggers.** A maneuver that's "always on" is a posture, not a maneuver. Maneuvers activate at specific moments — naming the trigger is essential.
- **Ignoring opponent's maneuvers.** Subject's maneuvers exist in a field where opponent also maneuvers. The most powerful subject maneuvers anticipate opponent's likely response (this is the bridge to `two-paths-synthesis` skill).

## Output template

```
─── STRATEGIC MANEUVERS ───

Subject: <name>
Current differential D: <from forces-and-potentials>
Available time horizon: <short/medium/long>

CANDIDATE MANEUVERS:

[Maneuver 1 — name]
Type: [Shape | Deception | Empty/Full | Shi | Third-Force | No-Fight]
Specific action: <concrete description, not abstraction>
Direction-shift if successful: D would shift from <current> to <projected>
Cost: <what subject expends>
Risk: <failure modes, probability rough>
Trigger: <what conditions activate this maneuver>

[Maneuver 2 — name]
[same structure]

[etc — 2 to 4 maneuvers total]

NOT APPLICABLE / WATCHED:
[Maneuvers explicitly considered but ruled out, with reason]
[Maneuvers marked "watch" — not active now but may become available]

RECOMMENDED COMBINATION:
<Primary maneuver + backup, with sequencing and triggers>
```

## Integration with deconstruction protocol

This skill operates in **Phase 3 of deconstruction (Staff Block)**, after `forces-and-potentials` and before `two-paths-synthesis`.

Pipeline: forces measured → maneuvers generated → two paths (formal + game-theoretic) compared, with maneuvers considered by both paths → forecast integrates the maneuver options into the decision tree.

This is what differentiates ОСП v2 from purely analytical intelligence: the staff produces not only "what will likely happen" but also "what subject can do about it." The latter is the actionable output for the owner.

Output is stored in Analysis record under `strategicManeuvers` field. Verification at horizon date checks: were the maneuvers actually applicable as predicted? Did subject pursue any of them? With what result?
