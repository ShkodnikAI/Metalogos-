---
name: patent-prosecution-analysis
description: Analyzes patent activity to reveal what companies are actually building, even when they're publicly silent. Patent filings expose R&D directions, technical priorities, anticipated competitive positioning, and timing. Companies file patents to protect their actual investments — patent portfolios are inadvertent disclosure of strategic direction. Skill provides systematic methodology to extract intelligence from public patent databases.
---

# Patent Prosecution Analysis — What Companies Actually Build

Companies are careful about public statements regarding R&D. They control announcements, manage expectations, time disclosures strategically. But they also file patents, and patents are public.

Patents reveal what companies are actually working on, what they consider valuable enough to protect, what technical approaches they're committed to, what competitive positioning they're anticipating. A company silent about a technology in public statements but filing dozens of patents on it is much more committed than one talking publicly without filing.

This skill is the discipline of reading patent activity as intelligence about real R&D priorities — often more reliable than corporate communications.

## Prerequisites

- Access to patent databases (USPTO, EPO, WIPO, Chinese patent office)
- Specific technology or company being analyzed
- Time investment for careful reading (patent quality varies enormously)

## Core principle

> Companies file patents on things they actually intend to build, value, or want to prevent competitors from building. Patent activity is involuntary disclosure of strategic direction. Reading patents systematically reveals technical priorities that public communications obscure.

## What patents reveal

**Direction of R&D investment:**
Cluster of related patents in narrow area = company committing significant resources there. Single isolated patent often less significant than concentrated cluster.

**Technical approach:**
Patents describe specific technical solutions. Reading them reveals which architectures, algorithms, materials companies are pursuing.

**Anticipated competition:**
Patents written defensively reveal what competitive moves the company expects. Patents on workarounds reveal what technologies the company is trying to enable.

**Timing:**
Patent filing timing is leading indicator. Filing typically precedes commercial deployment by 1-5 years (longer for hardware, shorter for software).

**Quality and depth:**
Number of patents matters less than quality. Fundamental patents (on core technologies) more valuable than incremental patents (on minor variations).

**Geographic strategy:**
Where patents are filed reveals market priorities. Filing in EU, China, Japan in addition to US suggests broader market intent.

## Patent quality assessment

Not all patents are equal. Quality indicators:

**High-quality patents:**
- Cited frequently by other patents
- Cited in academic literature
- Subject of licensing agreements
- Subject of patent litigation (offensive or defensive)
- Cover broad claims with strong technical specifications

**Low-quality patents:**
- Narrow incremental claims
- Defensive (filed to block competitors, not implement)
- Filed in isolation without cluster
- Heavy in legal language, light in technical substance
- Never licensed or asserted

For analytical purposes, focus on high-quality patents as signals.

## The procedure

### Step 1 — Define analytical question

What are you trying to learn from patents?
- What is company X actually building?
- What technical approach is winning in technology Y?
- What's company X's competitive positioning vs Y?
- What timeline for technology Z's commercialization?

Different questions require different analysis approaches.

### Step 2 — Define search parameters

For chosen analytical question:
- Companies (assignees) to search
- Technical classes (CPC codes) to search
- Time period
- Geographic offices (USPTO, EPO, WIPO)
- Inventors (sometimes most valuable signal)

Patent searches require domain knowledge. Generic keyword searches miss the technology because patents use specialized language.

### Step 3 — Conduct search

Use patent databases:
- USPTO Public Patent Search
- Google Patents
- Lens.org (academic)
- Specialized commercial databases (PatBase, Derwent) for deep analysis

Iterative refinement: initial search reveals related terms, refine to more specific search.

### Step 4 — Cluster results

Group patents by:
- Topic/sub-topic
- Technical approach
- Time period
- Inventor

Clusters reveal where investment is concentrated.

### Step 5 — Read key patents in detail

For top cluster, read 5-15 most relevant patents in detail. Note:
- Specific technical claims
- Cited prior art (reveals what they consider competition)
- Inventor profiles (reveal team composition)
- Filing timeline (reveals priority over time)

This is the time-intensive part. Cannot be shortcut to summary statistics.

### Step 6 — Cross-reference with other intelligence

Patent activity makes more sense alongside:
- Company financial reports (R&D spending, segment disclosures)
- Talent flow (who they're hiring)
- Public announcements (what they're claiming)
- Industry analyst reports

Divergence between patent activity and public statements is itself signal.

### Step 7 — Generate intelligence findings

Based on analysis:
- What is company actually building?
- What technical approaches are winning?
- What timeline for commercialization?
- What competitive moves anticipated?

### Step 8 — Output

Patent intelligence summary with implications.

## Worked example — Tesla autonomous driving (illustrative, 2024-2025 analysis)

**Question:** What is Tesla actually building for autonomous driving, despite mixed public statements?

**Search parameters:**
- Assignee: Tesla, plus subsidiaries
- CPC: G05D1 (controlling vehicles), G06N3 (neural networks), B60W30 (driving control)
- Period: 2020-2025
- Geographic: USPTO primarily, with EPO and Chinese cross-checks

**Cluster findings:**

**Cluster 1: End-to-end neural networks for driving**
- Substantial concentration of patents 2023-2025
- Specific neural network architectures
- Training methodology including simulation
- Inferred: Tesla committing to end-to-end approach (not modular pipeline like competitors)

**Cluster 2: Hardware-specific neural network optimization**
- Patents on FSD chip optimizations
- Quantization techniques
- Custom kernel implementations
- Inferred: Tesla optimizing software/hardware co-design tightly

**Cluster 3: Data collection and labeling**
- Methods for automatic labeling from fleet
- Specific edge case identification
- Training data pipeline patents
- Inferred: Tesla viewing data pipeline as competitive moat

**Cluster 4: Safety verification**
- Methods for measuring safety of autonomous systems
- Statistical approaches to safety validation
- Inferred: Tesla anticipating regulatory engagement around safety claims

**Cross-reference with public statements:**
- Public Tesla messaging emphasizes "end-to-end" approach
- Patent activity confirms this is real, not just rhetoric
- Talent recruiting confirms — high-profile hires aligned with these technical directions
- Capex spending pattern aligns

**Findings:**
- Tesla genuinely committed to end-to-end autonomous driving approach (not just rhetoric)
- Significant technical investment in approach
- Timeline for commercial deployment: patent filing pattern suggests 2025-2027 deployment of substantial improvements
- Competitive positioning: Tesla anticipates differentiation through data scale (fleet) and hardware integration

**Implications for ОСП analyses:**
- Investment thesis on Tesla autonomous capability has technical substance behind it
- Comparable analyses needed for Waymo, Cruise, Chinese players to assess relative positions
- Tesla's commitment is irreversible at this scale — strategic course locked in

## Anti-patterns

- **Patent count confusion.** Many patents ≠ much progress. Focus on quality, not quantity.
- **Reading patents superficially.** Patents are written in legal language obscuring technical content. Need domain expertise to extract meaning.
- **Single-source patents.** Single patent reveals one decision; cluster reveals strategy. Don't generalize from single patents.
- **Ignoring inventor flow.** Inventor changes between companies are signals. A team that filed for Company A now filing for Company B reveals migration.
- **Geographic blind spots.** Companies sometimes file in specific geographies for specific reasons. Limited geographic search misses signals.
- **Patent age confusion.** Patents have effective dates that may be much earlier than publication. Publication delay can be 18+ months. Recent patents reflect older decisions.
- **Defensive patent confusion.** Some patents are filed defensively to block competitors, not to implement. Distinguishing requires reading.

## Output template

```
─── PATENT PROSECUTION ANALYSIS ───

Subject: <company or technology>
Analytical question: <specific>
Date of analysis: <ISO>

SEARCH PARAMETERS:
- Assignees: <list>
- CPC codes: <list>
- Period: <range>
- Geographic: <list>

QUANTITATIVE OVERVIEW:
- Total patents found: <count>
- Patents in target clusters: <count>
- Quality-weighted assessment: <description>

CLUSTERS IDENTIFIED:

Cluster 1: <topic name>
  Patent count: <number>
  Time concentration: <when filed>
  Key inventors: <list>
  Technical approach: <description>
  Inference: <what this reveals>

Cluster 2: <same structure>

[2-5 clusters typical]

KEY PATENTS (read in detail):

Patent <ID>: <title>
  Specific claims: <summary>
  Significance: <description>

[5-15 patents]

CROSS-REFERENCE WITH OTHER INTELLIGENCE:
- Public statements: <consistent | divergent>
- Talent flow: <consistent | divergent>
- Financial disclosures: <consistent | divergent>
- Capex pattern: <consistent | divergent>

FINDINGS:
- Actual R&D priorities: <description>
- Technical approach being pursued: <description>
- Anticipated competitive moves: <description>
- Timeline for commercialization: <range>

IMPLICATIONS FOR ОСП:
- Investment thesis updates: <description>
- Watchlist additions: <list>
- Cross-references: <ОСП analyses to update>
```

## Integration with Лаборатория знаний

Used for:
- Major company analysis (FAANG-tier, leading research firms, key state actors)
- Specific technology trajectories where patents reveal direction
- Competitive intelligence for ОСП investment analyses

**Updated:** quarterly for tracked subjects; on demand for specific questions.

**Limitations:** patent analysis is time-intensive. Apply selectively where stakes justify investment.

Stored in KnowledgeArtifact with patent analysis findings linked to relevant tracked technologies.
