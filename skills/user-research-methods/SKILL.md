---
name: user-research-methods
description: Methods for understanding users and validating designs — heuristic evaluation, usability testing, competitive analysis, survey design. When to use which method. How to recruit, conduct, and synthesize findings. The discipline that turns assumptions into evidence.
---

# User Research Methods — Evidence Over Assumptions

Without research, designs are educated guesses. With research, designs are tested hypotheses. For some projects guesses are fine (you're the user, low stakes). For others, evidence is critical.

## Prerequisites

- Design or feature needs validation
- Owner has time/budget for research
- Decision dependent on research outcome

## Core principle

> Research isn't optional for high-stakes decisions. The cost of researching ($X, days) is small versus cost of building wrong thing (weeks, months, lost opportunity). The discipline is matching method to question — over-researching wastes time, under-researching wastes building effort.

## Method selection

Question → method:

**"What do users currently struggle with?"** → contextual inquiry (observe users in their environment), interviews

**"Can users complete this task with our design?"** → usability testing

**"Which design works better?"** → A/B test (if traffic) or comparative usability test

**"What do users want?"** → survey + interviews (combine for breadth + depth)

**"How does competition solve this?"** → competitive analysis

**"Is our design accessible?"** → heuristic evaluation + automated tools + manual accessibility audit

**"Why do users abandon at step X?"** → analytics + targeted interviews

## Heuristic evaluation

Fastest, cheapest method. Designer or evaluator reviews design against established heuristics. No users needed.

**Nielsen's 10 heuristics (classic, still useful):**

1. **Visibility of system status** — user always knows what's happening
2. **Match between system and real world** — uses user's language/concepts
3. **User control and freedom** — undo, escape from unwanted states
4. **Consistency and standards** — same things same way
5. **Error prevention** — better than error messages
6. **Recognition over recall** — show options, don't make user remember
7. **Flexibility and efficiency** — shortcuts for experts
8. **Aesthetic and minimalist** — irrelevant information competes with relevant
9. **Help users recognize and recover from errors** — clear language, suggested fixes
10. **Help and documentation** — searchable, task-focused

**Process:**
1. Walk through interface as user would for task
2. For each heuristic, note violations
3. Severity rate (1-4): cosmetic / minor / major / catastrophic
4. Suggest fix for each violation
5. Compile report

Time: 2-4 hours for typical screen.

Limitations: doesn't catch user-specific issues, dependent on evaluator expertise.

## Usability testing

Real users attempting real tasks with your design. Gold standard.

**Format:**
- 5-8 participants (Nielsen research: ~85% of issues caught with 5)
- Each session 30-60 min
- Moderated (researcher present) or unmoderated (recorded)

**Protocol:**
1. Brief intro (no leading info about what to look for)
2. Pre-task questions (background, context)
3. Tasks (3-5 representative tasks, ordered roughly easy to hard)
4. Think-aloud: ask user to verbalize thoughts
5. Post-task: difficulty rating, frustrations
6. Post-session: overall thoughts, open-ended

**During session:**
- Don't lead ("doesn't this button look obvious?" → biased)
- Don't help (let user struggle to see real friction)
- Note: where user paused, where they backtracked, what they ignored, where they verbalized confusion

**Tasks should be:**
- Realistic ("buy a pair of shoes" not "use the search filter")
- Open-ended ("find a way to share this" not "click the share button")
- Specific enough to verify completion ("with a 2-week return policy")
- Have measurable success ("user added it to cart")

**Analysis:**
- For each task: completion rate, time, errors, frustration markers
- Aggregate findings across users
- Pattern emerges from 2+ users showing same issue
- Rate issues by severity (catastrophic / serious / moderate / minor)
- Prioritize fixes

**Tools:**
- Maze (unmoderated)
- UserTesting (moderated/unmoderated)
- Lookback (moderated remote)
- Or low-tech: Google Meet + screen share + note-taking

## A/B testing

When you have traffic to split:
- Two versions of design
- Random users see version A or B
- Measure conversion / task completion / engagement
- Statistical significance threshold (typically p < 0.05)

**Considerations:**
- Need sufficient traffic for statistical power (calculate sample size beforehand)
- Run for at least a week (capture day-of-week effects)
- Don't change other variables mid-test
- Be honest about results: small differences may not be meaningful

**When NOT to A/B test:**
- Low traffic (results never significant)
- Drastically different designs (multiple variables at once)
- Critical safety/legal — can't ship knowingly-broken to half users
- When usability testing would answer faster

## Surveys

Use when:
- Need quantitative data
- Need to hear from many users
- Following up on qualitative research

**Question types:**

**Closed (quantitative):**
- Multiple choice
- Likert scale (Strongly Disagree → Strongly Agree, 1-5 or 1-7)
- Net Promoter Score (NPS)

**Open (qualitative):**
- "Tell us about a time when..."
- "What would make this easier?"
- "What's frustrating about current approach?"

**Survey design principles:**
- Short (5-10 min max)
- Clear language (no jargon)
- One question per question (compound questions confuse)
- Avoid leading ("how amazing was this?" vs "how was this?")
- Mix open and closed
- Test internally before sending

**Distribution:**
- Email list
- In-app prompt
- Social media
- Incentive often helps response rate (gift card, etc.)

**Analysis:**
- Quantitative: percentages, averages, distributions
- Qualitative: thematic coding (group similar answers)

## Competitive analysis

Systematically review competitor solutions.

**Process:**
1. Identify competitors (direct + indirect)
2. For each: capture screenshots, workflows, key UI patterns
3. Note: what works well? what works poorly?
4. Compare against your design
5. Identify gaps, opportunities, standards-of-domain

**Don't blindly copy.** Competitor design has constraints/decisions you don't see. But knowing what users are used to is valuable.

## Accessibility audit

Beyond automated tools (Lighthouse, axe), manual audit:

- Test with keyboard only (no mouse)
- Test with screen reader (VoiceOver/NVDA)
- Test at 200% zoom
- Test with high-contrast mode
- Test with reduced motion
- Test color-blind simulation

For Fosved: see `accessibility-first` for full discipline.

## Synthesis

Findings → action.

**Affinity mapping:**
- Each finding on sticky note (physical or digital)
- Group by theme
- Themes prioritized by frequency + severity

**Severity rating:**
- Catastrophic: blocks task completion entirely
- Serious: significant friction, recovery needed
- Moderate: noticeable, but doesn't block
- Minor: nice-to-fix

**Action items:**
- Each finding → specific change OR explicit "won't fix" decision
- Track in DesignArtifact iterations or DefectRecord

## Documentation

Findings recorded in `UserResearchSession`:

```typescript
{
  sessionType: 'usability_test',
  topic: 'Mobile checkout flow',
  methodology: '5 participants, moderated remote, think-aloud, 6 tasks each',
  participants: 'Anonymized: 3 desktop-first, 2 mobile-primary, mix of techie/non-techie',
  findings: '...',  // structured findings
  recommendations: '...',
  conductedAt: ...
}
```

Searchable archive of what's been validated.

## Research cadence for Fosved

For Fosved owner's own use: low research need (you ARE the user, you know yourself).

For client projects: research scaled to project criticality.
- Quick feature: heuristic eval suffices
- Significant project: usability test before launch
- Major product: ongoing research throughout

For AI office products: heuristic eval on flow + accessibility audit minimum.

## Anti-patterns

- **Research as theater.** "We did research." But findings not acted on. Waste.
- **Leading questions.** "How great is this?" Confirms what you want to hear.
- **Too few participants.** 1 user's frustration ≠ pattern. Need 3+ saying similar.
- **Wrong participants.** Tested with techies, product for novices. Generalization invalid.
- **Friends and family bias.** "My friends like it." They're nice. Not representative.
- **A/B testing what's obviously broken.** Don't need data to know broken is bad.
- **Survey design fishing.** Asking questions to confirm pre-determined narrative.
- **Ignoring qualitative.** Only counting numbers. The "why" is in interviews.
- **Ignoring quantitative.** Only stories. Doesn't scale, hard to prioritize.
- **Done once, never again.** Users evolve, designs evolve. Periodic re-research.
- **Findings without action.** Report sits in drawer. Real cost: research time + missed improvement.

## Integration

- Triggered by `/design-research` command
- Outputs feed back to `DesignArtifact` for iteration
- `dev-handoff-specs` reflects findings
- `qa/integration-testing-patterns` may verify research-driven changes
- Pattern: research → design → impl → verify → repeat
