---
name: competing-claims-comparison
description: Compares the project's claims directly against competing projects pursuing the same goal. When multiple teams target same problem, comparison surfaces what's actually distinctive vs what's universal claim, who has the strongest evidence vs the strongest pitch, and where this project sits in the actual competitive landscape. Output: comparative table identifying real differentiation and competitive positioning.
---

# Competing Claims Comparison — Where Does This One Actually Stand

The pitch presents the project as unique. Competing projects typically aren't mentioned, or are mentioned dismissively. The actual competitive landscape often shows multiple teams pursuing similar goals with similar approaches — sometimes one project's "breakthrough" is another's incremental improvement.

This skill is the discipline of mapping the actual competitive landscape to position the project realistically.

## Prerequisites

- `rapid-domain-immersion` providing field knowledge
- `project-deconstruction` providing project shape
- Access to public information about competitors

## Core principle

> Most technical fields have multiple teams pursuing similar goals. The pitch usually presents the project as either unique or dismissively-different from named competitors. Real competitive analysis often shows the project is one of several similar approaches with debatable advantages. Comparing claims directly reveals real positioning vs marketed positioning.

## What to compare

For each competing project, gather:

**Approach:**
- Technical approach used (which school/method)
- Specific architectural choices
- Differentiation claims

**Status:**
- Stage of development
- Demonstrated capabilities
- Notable achievements
- Funding raised
- Team size and quality

**Performance claims:**
- Same metrics if possible (otherwise translate)
- Demonstrated vs claimed
- Trajectory of improvement

**Resources:**
- Funding visible
- Team strength
- Infrastructure
- Partnerships

**Timeline:**
- Claimed milestones with dates
- Track record of meeting/missing prior milestones

## The procedure

### Step 1 — Identify competitors

For the project's goal, who else is pursuing it?

Sources:
- Industry analyst reports (CB Insights, Pitchbook, specialized analysts)
- Conference proceedings (who's presenting in same sessions)
- Patent landscapes (who's filing in same area)
- VC portfolio sites (which firms invest in space, what else they fund)
- Academic literature (which research groups work in area)
- Government R&D databases (NIH RePORTER, NSF, ARPA-E)

Aim for 5-10 competitors. Less misses key players; more dilutes attention.

### Step 2 — Tier competitors

Not all competitors are equal. Tier them:

**Tier 1 — Direct competitors:** same approach, same goal, same maturity
**Tier 2 — Adjacent approaches:** different approach, same goal
**Tier 3 — Peripheral:** related but distinct goal or different maturity

Focus mainly on Tier 1, with awareness of Tier 2-3.

### Step 3 — Build comparison matrix

For comparable dimensions across competitors, build matrix:

```
Dimension          | Project | Comp 1 | Comp 2 | Comp 3 | Best
Funding raised     | $X     | $Y    | $Z    | $W    | Comp Y
Team size          | N      | M     | P     | Q     | Comp M
Stage              | beta   | prod  | beta  | alpha | Comp Y
Core metric 1      | A      | B     | C     | D     | Best one
[etc.]
```

Surface where project actually stands.

### Step 4 — Identify real differentiation

If project differs meaningfully from competitors:
- What specifically is different?
- Is the difference advantageous? (Not just different — actually better in deployment scenarios)
- Is the advantage durable or replicable?

If project doesn't actually differ:
- What's the basis for claimed differentiation?
- Is project a faster follower? Slower follower? Equal participant?

### Step 5 — Assess competitive positioning

Where does project actually sit?

**Leader:** ahead on most metrics, deserves premium evaluation
**Strong contender:** in top quartile, defensible
**Average:** middle of pack, requires specific advantage
**Trailing:** behind, requires major catch-up
**Niche:** unique positioning may justify even if behind on aggregate

### Step 6 — Generate comparison-based combat questions

Ask the project to position relative to specific competitors:

"Tokamak Energy is targeting similar HTS-magnet compact tokamak; their reported magnet performance is X. How does yours compare?"

These questions are powerful because:
- Force specific comparison rather than generic claims
- Reveal whether team is aware of competition
- Reveal whether team has actual advantage or just claimed one

### Step 7 — Output competitive analysis

Comparison matrix + positioning + combat questions.

## Worked example — fusion startup competitive landscape

For FusionCorp (compact tokamak with HTS magnets), competitors:

**Tier 1 — Direct (same approach):**
- Commonwealth Fusion Systems (SPARC) — most directly comparable, $2B+ raised, MIT spinout, very strong team
- Tokamak Energy (UK) — similar approach, smaller scale, growing
- ENN (China) — Chinese effort with similar magnet approach

**Tier 2 — Adjacent fusion:**
- Helion Energy — different fuel cycle (D-He3), different approach
- TAE Technologies — alternative confinement
- General Fusion — magnetized target fusion
- ITER — public mega-project

**Tier 3 — Peripheral:**
- NIF (laser fusion) — completely different physics
- Fission startups (NuScale, etc.) — different technology

### Comparison matrix:

```
Dimension              | FusionCorp | CFS (SPARC) | Tokamak Energy
Funding raised         | $300M      | $2B         | $400M
Team size              | 80         | 250         | 120
HTS magnet demo        | small scale| 20-Tesla demo (2021) | 20+ Tesla R&D
Plasma demonstration   | 30s pulse  | not yet     | deuterium plasma
Claimed first plasma   | 2027       | 2025-2026   | 2025
Claimed Q=10           | 2027       | 2026-2027   | 2026-2027
Claimed commercial     | 2030       | 2032-2035   | 2030s
Backing                | private    | Bill Gates et al, premier MIT | UK gov + private
```

### Real differentiation analysis:

**Vs CFS:** CFS is well ahead on funding, team, and HTS demonstration. FusionCorp would need specific technological advantage to justify position. From public materials, advantage isn't apparent.

**Vs Tokamak Energy:** comparable size, ahead on HTS R&D. FusionCorp is a fast follower at best.

**Vs Helion (Tier 2):** if Helion's D-He3 approach succeeds, the entire DT fusion path becomes obsolete. That's risk to FusionCorp's positioning beyond direct competition.

### Competitive positioning:

FusionCorp is **trailing on most measurable dimensions** vs CFS and Tokamak Energy. Their pitch emphasizes uniqueness; reality is they're a less-funded, less-developed entrant in crowded compact-tokamak space.

This doesn't mean the company can't succeed — sometimes well-executed late entrants pass earlier leaders. But valuation should reflect competitive position.

### Comparison-based combat questions:

1. "CFS achieved 20-Tesla HTS magnet demonstration in 2021. What's your magnet demonstration status, and what's your specific advantage over their approach?"

2. "Tokamak Energy is similar size, similar approach, with substantial UK government backing. What makes you a stronger investment than them?"

3. "Commonwealth Fusion has $2B raised and a 250-person team. You have $300M and 80 people. How do you compete on resources for what's fundamentally a capital-intensive race?"

4. "If Helion's D-He3 approach works, your tritium and materials problems become moot. How do you assess their probability of success and what does it mean for your roadmap?"

5. "ITER is the established mega-project. What does ITER demonstrate that you build on, and what does ITER prove that you avoid?"

## Anti-patterns

- **Competitor blindness.** Many pitches ignore competitors entirely. Discipline is finding them whether or not pitch acknowledges.
- **Single-competitor focus.** "We're better than X" — but Y, Z, W also exist. Mapping full landscape matters.
- **Apples to oranges.** Comparing different metrics or different stages misleads. Use comparable dimensions where possible; explicit translation where not.
- **Static analysis.** Competitors evolve. Recent updates matter; year-old competitor data may mislead.
- **Overweighting visible competitors.** Stealth competitors and adjacent approaches matter. Public visibility isn't competitive position.
- **Ignoring resource asymmetries.** Sometimes the "less developed" project has more resources to catch up. Vice versa.

## Output template

```
─── COMPETING CLAIMS COMPARISON ───

Project: <identifier>
Goal/space: <description>

COMPETITORS IDENTIFIED:

Tier 1 (direct):
- <Competitor>: brief profile

Tier 2 (adjacent):
- <Competitor>: brief profile

Tier 3 (peripheral):
- <Competitor>: brief profile

COMPARISON MATRIX:
[table comparing project to top competitors on standardized dimensions]

REAL DIFFERENTIATION ASSESSMENT:
- Vs Tier 1 leader: <project's position>
- Vs other Tier 1: <position>
- Specific durable advantage: <if any>

COMPETITIVE POSITIONING:
[Leader | Strong contender | Average | Trailing | Niche]
Justification: <reasoning>

COMPARISON-BASED COMBAT QUESTIONS:
1. <Question forcing comparison>
2. [more]
[3-5 typical]

INVESTMENT IMPLICATIONS:
- Position justifies premium valuation: [yes/no/specific conditions]
- Catch-up potential: <assessment>
- Risks from competitive dynamics: <list>
```

## Integration with Expert protocol

Tier 2 — invoked for:
- Investment-context evaluations
- Crowded competitive landscapes
- When project claims uniqueness that needs verification

Output integrates into combat questions.

Stored in ExpertBriefing under `competingClaimsComparison`.

Particularly valuable for VC-style due diligence where the question "is this a winner?" is fundamentally about competitive positioning, not just absolute capability.
