---
name: pre-meeting-intel
description: Tier 3 special technique. OSINT-style intelligence gathering on specific people who will attend the meeting — their academic background, prior projects (successes and failures), patents, publications, professional reputation, possible conflicts of interest. Helps owner know who at the table actually has expertise, who's there for credibility, who has motivations beyond the technical. Used for high-stakes meetings where understanding the people matters as much as understanding the technology.
---

# Pre-Meeting Intel — Knowing Who You're Actually Talking To

The meeting has people, not abstractions. Who they are determines what they'll say. The CEO defending a project they've staked their career on speaks differently than a hired consultant. The Nobel laureate advisor lending credibility but not actually working on the project speaks differently than a senior researcher who actually built it. The investor relations person speaks differently than the CTO.

This skill is the OSINT-style discipline of knowing who's at the table before the meeting. Public information about the people often reveals as much as public information about the technology — sometimes more.

## Prerequisites

- Names and ideally roles of meeting attendees
- Time to research (this is Tier 3 because of effort required)
- Owner authorization for OSINT-level effort
- Ethical boundaries respected (public sources only)

## Core principle

> Meetings have people. People have histories. Public histories often reveal what the meeting itself won't — actual expertise levels, prior project outcomes, possible conflicts of interest, professional standing. The skill is gathering and interpreting these public signals to inform the owner about who's actually in the room.

## What to research per person

For each meeting attendee, gather from public sources:

### Academic/professional background

- Education (institutions, degrees, dates)
- Career path (roles, employers, durations)
- Significant transitions (founder/exit, lab moves)
- Current role and tenure

### Substantive expertise

- Publications (peer-reviewed, preprints, citations)
- Patents (filed, granted, assigned)
- Conference talks and presentations
- Specific projects led or contributed to
- Specific technical domains of expertise

### Track record

- Prior projects: outcomes (success, failure, ongoing)
- Prior companies: status (acquired, IPO, shut down, ongoing)
- Prior predictions: did they come true?
- Public statements over time: consistency or shifts

### Professional reputation

- Citations and citations-in-context
- Mentions in industry analyses
- Comments by other researchers (named)
- Awards or recognitions (and what they mean)
- Critical mentions or controversies

### Network and affiliations

- Co-authors, collaborators
- Board memberships
- Advisory roles (paid? unpaid?)
- Investments
- Foundation involvement

### Possible conflicts of interest

- Financial interests in current project
- Investments in competitors
- Family or close professional ties
- Past positions that might bias

## The procedure

### Step 1 — Get attendee list

If unavailable, ask owner. Names + roles ideally; just names work but roles add context.

### Step 2 — Categorize attendees

Different attendees serve different roles in meetings:
- **Technical lead** — actually built the technology
- **CEO/Business lead** — runs the company, may not be technical
- **Advisor** — credibility lend, may or may not be substantively involved
- **Investor relations / fundraising** — there to sell
- **Academic credentialed** — provides scientific authority
- **Customer/partner** — there to validate, may have interests
- **Government/institutional** — there to evaluate or coordinate

Understanding who serves what role shapes how to interpret what they say.

### Step 3 — For each priority attendee, conduct research

Spend more time on key technical decision-makers; less on attendance-only roles.

Sources:
- **Google Scholar** — publications and citations
- **ORCID, ResearchGate** — academic profiles
- **LinkedIn** — career history (note: people curate this)
- **Patent databases** — USPTO, EPO inventor searches
- **Crunchbase** — for company history
- **Twitter/X, Mastodon, blogs** — public statements
- **University faculty pages** — official affiliations
- **Conference programs** — recent talks
- **Government databases** — NIH RePORTER for funding

### Step 4 — Identify substantive vs theatrical roles

For each person, distinguish:
- **Substantive role:** they contribute knowledge/work to project
- **Theatrical role:** they provide credibility but aren't substantively involved

This distinction matters for interpreting their answers. The Nobel laureate advisor may not actually understand current project specifics; the senior researcher who built it does.

### Step 5 — Track record analysis

For each substantive person, what's their track record?

- Have prior projects succeeded? Specifically, prior projects in similar domain?
- Have prior predictions come true?
- Have they overpromised in past presentations?
- Have they ever publicly acknowledged failures?

Pattern of past behavior predicts current behavior.

### Step 6 — Conflict of interest check

- Financial interests in this project
- Competing investments or roles
- Family/personal relationships that might bias
- Past statements that might constrain current ones

### Step 7 — Generate person-specific tactical notes

For each priority attendee:
- What expertise they actually have (vs claimed)
- What questions they're best positioned to answer
- What questions they may deflect on
- What conflicts of interest to keep in mind
- What track record patterns suggest

### Step 8 — Output intel briefing

Per-person profiles + tactical notes. This complements technical briefing.

## Worked example — meeting with FusionCorp leadership

Hypothetical attendees:
1. Dr. Alice Chen — CEO and co-founder
2. Dr. Bob Smith — CTO and co-founder
3. Prof. Carol Williams — Senior Scientific Advisor
4. Mr. David Lee — Head of Investor Relations
5. Dr. Eric Park — VP Engineering

### Per-person intel:

**Dr. Alice Chen (CEO):**
- Background: PhD plasma physics MIT, 2010. Postdoc at PPPL. Senior scientist at TAE 2014-2019. Founded FusionCorp 2020.
- Publications: 30+, mostly during academic career
- Patents: 12, all assigned to TAE or FusionCorp
- Track record: TAE work was on field-reversed configuration; pivoting to compact tokamak shows flexibility but also potentially commitment-shopping
- Prior predictions: at TAE, predicted commercial fusion by 2025; obviously hasn't materialized
- Reputation: technically credible, ambitious timelines critique noted by colleagues
- Tactical: substantive technical knowledge, but past pattern of aggressive timelines should anchor expectations on FusionCorp's claims

**Dr. Bob Smith (CTO):**
- Background: PhD MIT magnet engineering 2015, postdoc Commonwealth Fusion 2015-2019, then FusionCorp
- Publications: 15, mostly on HTS magnet engineering
- Patents: 8, mix of CFS-assigned and FusionCorp-assigned
- Track record: solid engineering credentials in core relevant area; left CFS to start similar approach
- Reputation: good engineer, somewhat junior leadership-wise
- Tactical: best person to ask detailed magnet questions; leaving CFS for similar venture is interesting (why?)

**Prof. Carol Williams (Advisor):**
- Background: distinguished plasma physicist, Princeton emeritus
- Publications: 200+, including foundational tokamak work 1980s-1990s
- Recent: not specifically on compact tokamak / HTS approach
- Conflicts: equity in FusionCorp (advisor stake), possibly other fusion advisory roles
- Tactical: theatrical role likely — provides credibility but may not have detailed knowledge of FusionCorp specifics. Don't direct technical questions to her; don't be impressed by her presence as evidence of project quality.

**Mr. David Lee (IR):**
- Background: MBA, prior at McKinsey energy practice
- No technical background
- Tactical: there to sell, ignore for technical content. Useful for deal terms only.

**Dr. Eric Park (VP Engineering):**
- Background: PhD engineering at Stanford 2018, joined FusionCorp 2022
- Publications: 5, mostly engineering
- Patents: 3, all FusionCorp
- Track record: relatively early career, built credibility at startups before
- Tactical: best person for engineering implementation questions. Junior enough that may answer honestly when CEO would deflect.

### Tactical notes:

- For technical depth, address questions to Smith (CTO) and Park (VP Eng), not Williams (advisor)
- Watch for CEO's history of aggressive timelines — discount commercial timeline claims accordingly
- The Smith-from-CFS angle is significant — "Why did you leave CFS to start similar approach?" is a meaningful question
- Williams' presence is credibility-theater; don't let her presence constrain technical scrutiny
- Lee's IR background means business-narrative is well-rehearsed; assume sophisticated framing of business case

## Anti-patterns

- **Privacy violations.** Public sources only. Don't research personal lives; focus on professional.
- **Single-source profiles.** LinkedIn is curated; cross-reference with publications, patents, etc.
- **Old data.** Career profiles age. Verify current affiliations and recent activity.
- **Conflict-of-interest interpretation as accusation.** Conflicts exist; recognizing them is analytical, not accusatory.
- **Equating presence with expertise.** Senior advisors lend names; doesn't mean they're substantively involved.
- **Over-relying on credentials.** PhD from prestigious school doesn't guarantee current expertise in specific area. Track record matters.
- **Adversarial framing during meeting.** Intel informs interpretation; doesn't license confrontation.

## Output template

```
─── PRE-MEETING INTEL ───

Meeting: <description>
Date: <ISO>
Attendees researched: <count>

[For each attendee:]

ATTENDEE: <name>
Stated role: <as represented>
Background: <education, career>
Substantive expertise: <specific>
Track record: <prior projects, predictions, outcomes>
Reputation: <from public mentions>
Conflicts of interest: <list>
Substantive vs theatrical: <classification>
Tactical notes: <how to interpret what they say, what to ask them>

[Repeat for each priority attendee]

OVERALL TACTICAL ASSESSMENT:
- Best technical contact: <person>
- Best business contact: <person>
- Theatrical credibility presence: <person>
- Person to push hardest: <reasoning>
- Person to engage carefully: <reasoning>
- Conflict-of-interest watchlist: <items>
```

## Integration with Expert protocol

Tier 3 — invoked for:
- High-stakes meetings (large investment, strategic decision)
- When attendees include known public figures
- When prior project history would inform interpretation

Output is **separate document** from main briefing, supplementary. Owner reads main briefing for tech; reads intel for people; integrates both during meeting.

Stored in ExpertBriefing under `preMeetingIntel`.

Ethical boundary: public sources, professional information only. No personal life research, no surveillance, no extraction beyond publicly available.
