---
name: reference-sheet-composition
description: Techniques for producing dense one-page reference sheets (Type 4) — for Clients, Projects, KnowledgeArtifacts. Like military mission cards or Wikipedia infoboxes — high information density, scan-friendly, print-ready. The viewer returns to a reference sheet repeatedly over weeks/months for quick reminder.
---

# Reference Sheet Composition — Dense One-Page References

A reference sheet is NOT an infographic for sharing. It's a **personal reference card** the owner returns to for quick reminder about an entity (client, project, technology). High density, dry layout, no narrative — just facts in scannable structure.

This is closer to military operations card or aircraft cockpit reference than to marketing infographic.

## Prerequisites

- `visual-grammar` loaded
- `infographic-templates` understood (knows about Type 4)
- Entity has accumulated data (timeline, projects, financial events, etc.)

## Core principle

> A reference sheet must answer "tell me everything important about X in 30 seconds of scanning". Not "tell me the most important thing about X" (that's Type 2 Memorable). The viewer already knows X exists; they need refresh on details. Density matters more than impact.

## Layout philosophy

Reference sheets follow **information density patterns** from print design:

- **Newspaper layout** — clear columns, distinct sections, easy to scan
- **Wikipedia infobox** — left/right columns, key-value pairs, condensed but readable
- **Military reference card** — color-coded by criticality, mono font for data, clear emergency info

NOT visual storytelling. Not narrative. Reference data, ordered for retrieval speed.

## Required sections (any entity type)

Every reference sheet has these zones:

### 1. Header bar (full width, ~100px)

**Contents:**
- Entity name (H1) — left
- Entity type / category (H3) — center
- Status badge — right
- Last updated date — bottom-right corner

Always same location, regardless of entity. Owner builds muscle memory.

### 2. Key facts grid (top third, 3-4 cards)

Visual cards (3-4 columns), each ~400×300px:
- Each card has its own subsection header (H3)
- 3-5 key-value pairs
- Numbers in IBM Plex Mono for column alignment
- Important values in gold accent

Card content varies by entity:
- **Client:** contact, financial, projects, tier history
- **Project:** team, deadlines, client, dependencies
- **Technology:** hype position, disruption metrics, key players, competitive landscape

### 3. Timeline strip (middle horizontal band)

Last 10-15 events compressed into horizontal timeline.

- Date markers above line (IBM Plex Mono 12px)
- Event descriptions below line (Inter 11px italic)
- Color coding: gold (financial events), forest (positive milestones), burgundy (issues), navy (neutral)

Timeline is the **temporal scan** dimension — owner sees "what's happened recently" in one glance.

### 4. Details panel (bottom third)

Less critical info but still useful:
- Notes
- Tags
- Related entities
- Annotations from owner

Can be text-heavy. Smaller font (Inter 12-14px).

### 5. Footer (full width, ~80px)

- Entity ID
- Created date
- Last enriched / scanned date
- Tags / hashtags

## Density principles

Reference sheet is **DENSE by design**. Goal: pack maximum useful info in minimum space without becoming unreadable.

### Density tactics

**Compress text:**
- Use abbreviations consistently (Q1, MTD, YTD, $M, k, B)
- Drop articles in labels ("Revenue" not "The Revenue")
- Use icons + text rather than text alone for known categories

**Use mono font for numerical columns:**
All numbers in IBM Plex Mono. They align by digit, scan instantly.

**Color-code by category, not decoration:**
Financial = gold accent. Project = forest. Risk = burgundy. Personnel = navy. Once internalized, viewer reads color before reading text.

**Use small multiples for similar items:**
6 invoices? Show as compressed grid of 6 small cards, not a paragraph of text.

**Tables, not paragraphs:**
Structured data → table. Avoid sentences for data.

## What NOT to do (density anti-patterns)

- **Storytelling.** Reference sheet is not narrative — it's database view.
- **Visual hierarchy via size.** Don't make ONE element huge. All cards should be similar in importance — viewer picks what they need.
- **Lots of negative space.** Memorable visuals need space; reference sheets need density. Use space only as separators between cards.
- **Decorative elements.** Even more strict than other types — every pixel is data.
- **Long captions.** Captions <80 chars, ideally <50.
- **Colorful headers.** Headers stay navy. Color is for data, not structure.

## Print-readiness

Reference sheets should print well on A4 (210×297mm) at 72-150 DPI.

Implications:
- Canvas 1200×900 SVG renders well at A4 landscape
- Font sizes shouldn't go below 10pt printed (which is ~13px in 1200×900 viewBox)
- Use solid fills, not gradients (gradients print poorly)
- High contrast (cream background, navy text) — readable on grayscale print if color unavailable

## Specific templates

### Client Reference Sheet

```
Header (1200×100):
  [Name] — H1, navy
  [Company] — H3, grey, beside name
  [Tier badge: T1/T2/T3] — top right, color graded
  [Status: active/dormant/lost] — top right

Cards row (1200×300, 3 cards):
  Card 1 — Contact:
    Email | Phone | Address | Time zone | Primary language
  Card 2 — Financial:
    Total revenue | Open invoices | Avg payment days | YTD trend
  Card 3 — Projects:
    Active count | Completed count | Last project | Next planned

Timeline (1200×300):
  Horizontal line with last 10-15 events
  Color-coded by type
  Date markers

Footer (1200×100):
  Created | Last enriched | Tags | Compiled profile flag
```

### Project Reference Sheet

```
Header (1200×100):
  [Name] — H1
  [Type: razovyy/podpiska/komanda] — H3 beside
  [Status: planning/active/done] — top right
  [Progress: 65%] — gauge in top right

Cards row (1200×300, 3 cards):
  Card 1 — Team:
    Lead | Members | External collaborators | Roles
  Card 2 — Timeline:
    Start | Deadline | Days remaining | Milestones hit
  Card 3 — Client:
    Client name | Annotation | Initial brief | Communication channel

Tasks panel (1200×300):
  Open tasks sorted by priority (red/amber/green)
  Each: title | assignee | deadline | status

Footer: created, updated, project_id, hashtags
```

### Technology Reference Sheet (from KnowledgeArtifact)

```
Header: name | category | status | last scanned | hype position summary

Cards row:
  Card 1 — Hype cycle:
    Position on Gartner curve (small visual gauge)
    Time in current position
    Direction of movement
  Card 2 — Disruption metrics:
    Probability | Timeline | Velocity | Inflection?
  Card 3 — Key players:
    Top 5 companies/labs with tier classification

Profile section (1200×300):
  Compressed technology profile in 6-8 bullet points
  Key differentiators

Footer: lastScanned, watchlist status, related artifacts
```

## Worked example — Client "FusionCorp"

Header:
- "FusionCorp" — H1 navy
- "Investment-stage compact fusion startup" — H3 grey
- T2 badge (tier 2)
- "Active" — forest accent

Card 1 — Contact:
```
Email:       alex@fusioncorp.io
Phone:       —
Location:    Boston, MA
Language:    EN | RU
```

Card 2 — Financial:
```
Status:      Potential investment ($5M)
Action:      Due diligence in progress
Last meet:   2026-05-08 (academy of sciences)
Decision:    Pending
```

Card 3 — Related:
```
Briefings:   3 (BRF-007, BRF-014, BRF-018)
Analyses:    1 (ANL-042 fusion industry context)
Documents:   2 (NDA-2026-03, MSA-pending)
Tags:        #fusion #tokamak #investment_due_diligence
```

Timeline:
```
[●─────●──────●────●────●─●─●───────●]
2025  2026Q1  Q2   Q3                    
       ↑      ↑    ↑    ↑↑↑       ↑
   intro  Brf-7 NDA Brf-14 academy Brf-18
```

Footer: Created 2025-09-12 | Last enriched 2026-05-09 | Compiled profile: yes | T2 confirmed 2026-02

## Anti-patterns

- **Making reference sheet look like memorable infographic.** Wrong purpose. Density over impact.
- **Skipping fields because "they're empty".** Show "—" placeholder. Consistency matters for visual scan.
- **Differentiating cards visually beyond category.** All cards in a row look structurally same; only content differs.
- **Drama in colors.** No splash colors. Restrained palette use; gold ONLY for hero numbers.
- **Long descriptions in cards.** Cards have key-value pairs, not paragraphs. If paragraph needed, it goes in "Notes" section in details panel.

## Integration

- Used by `lib/visual.js` `generateReferenceSheet`
- Triggered by `/visual reference <type> <id>` command
- Aggregates data across multiple Prisma models (Client + Invoices + Projects + Timeline = one sheet)
- Renders to SVG, printable to PDF if needed
