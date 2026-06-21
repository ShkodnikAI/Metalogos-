---
name: cognitive-laws-application
description: Applied cognitive psychology laws for UI design — Fitts, Hick, Miller, Aesthetic-Usability Effect, Von Restorff, Jakob's Law, Doherty Threshold, Serial Position Effect. Not abstract psychology — concrete design rules with measurable applications. The foundation that explains WHY good interfaces work.
---

# Cognitive Laws Application — Psychology That Designs Interfaces

Cognitive laws are observed regularities in human perception and behavior. They've been measured in labs and validated in production designs for decades. Knowing them is the difference between guessing and engineering.

This skill is a working catalog: each law with its formulation, design implication, and Fosved-specific application.

## Prerequisites

- `user-task-analysis` complete (know what user is trying to do)
- `visual-grammar` from Visual Department (brand identity)

## Core principle

> Cognitive laws are universal — they apply regardless of project, audience, or aesthetic. Ignoring them creates interfaces that feel "off" without anyone knowing why. Applying them deliberately produces interfaces that feel natural — because they match how brains actually work.

## The catalog

### Fitts's Law — target size and distance

**Formulation:** time to acquire a target is a function of distance to target and size of target. Bigger and closer = faster.

**Formula:** T = a + b × log₂(D/W + 1) where D=distance, W=width.

**Design implications:**
- **Critical actions need big targets.** Primary CTA button: minimum 44×44px on touch (88×88 ideal), 24×24px on mouse.
- **Frequently used controls go near where the user is looking.** Don't put "Save" 800px from the edit area.
- **Edge and corner pixels are infinitely accessible** on desktop (no edge to overshoot). Apple uses this — menu bar at screen edge.
- **Touch targets need 8px+ spacing** between adjacent targets. Otherwise mis-tap risk.

**Fosved application:**
- Primary action button (e.g., "Send", "Save", "Analyze") on each screen: minimum 44×44 on mobile, prominent location.
- Adjacent buttons (e.g., "Cancel"/"Confirm" pair): 12-16px gap minimum.
- Don't put critical actions in screen corners (touch overshoot risk on phones).

### Hick's Law — choice complexity

**Formulation:** time to make decision increases logarithmically with the number of choices.

**Formula:** T = b × log₂(n+1) where n=number of equally-probable choices.

**Design implications:**
- **Reduce option count.** 5 options = ~2.3 units of decision time. 10 options = ~3.5. 50 options = ~5.7.
- **Group choices** when many options needed. Categories reduce effective n.
- **Highlight primary choice** — "recommended" badges shift user's effective choice space.
- **Progressive disclosure** — show core options first, advanced on demand.

**Fosved application:**
- Yana's main menu: 6-8 specialist commands visible, others discoverable on `/help`. Not 30 commands at once.
- Settings screens: tabs/groups, not flat list of 30 settings.
- Decision UIs (pick a plan, pick a method): 3-5 options preferred, never 10+ flat.

### Miller's Law — working memory

**Formulation:** human short-term memory holds about 7±2 items.

**Design implications:**
- **Chunk information** into groups of 3-5 (safe) or up to 7 (max).
- **Phone numbers, codes, IDs** — chunked for memorability (123-456-7890 not 1234567890).
- **Navigation menus** — 7 or fewer top-level items.
- **Tabs** — 5-7 max. More = overflow indicator.
- **Form fields** — group related fields (address: 3-5 fields together).

**Fosved application:**
- Each specialist's commands: groups of 3-5 typical, not unstructured list.
- Multi-step flows: progress indicator with steps named (chunks user can hold in memory).
- ADR template: 5 sections (context, decision, alternatives, consequences, retrospective) — fits working memory.

### Aesthetic-Usability Effect

**Formulation:** users perceive aesthetically pleasing designs as more usable.

**Implication:** polish matters — even if functionality identical, polished version rated more usable. Users forgive minor issues in beautiful designs.

**But:** doesn't mean style over substance. Means polished+functional > unpolished+functional.

**Design implications:**
- **Visual consistency** is functional benefit (not just aesthetic).
- **Quality finish** on small details (kerning, alignment, spacing) — affects perceived usability.
- **Don't ship ugly to "fix later"** — ugly version sets baseline expectations.

**Fosved application:**
- Brand identity consistency across all UIs (Inter Tight, navy/cream/gold, 8px scale).
- Polish details: button border-radius consistent, icon sizes consistent, spacing matches scale.
- Telegram bot messages — typography matters (proper Markdown formatting beats raw text).

### Von Restorff Effect (Isolation Effect)

**Formulation:** an item that stands out from similar items is more likely to be remembered.

**Design implications:**
- **One thing prominently different** = users notice and remember it.
- **Primary CTA** stands out from secondary actions (gold for primary on neutral background).
- **Error states** must stand out (burgundy + icon + position).
- **Don't make everything prominent** — defeats the effect.

**Fosved application:**
- Gold accent in Visual Department palette: used ONLY for hero data, max 1-2 elements per visual.
- Primary action in dev-handoff-spec: prominent, secondary actions muted.
- In OSP Analysis cards: most-likely scenario gets visual highlight, others equal-weighted.

### Jakob's Law — familiarity

**Formulation:** users spend most of their time on other sites/apps. They expect yours to work like the others they know.

**Design implications:**
- **Follow conventions** unless you have strong reason to deviate.
- Logo top-left → home. Menu icon → navigation. ✕ → close. ⚙ → settings.
- **Novel UI is expensive** — users must learn it. Worth only when payoff justifies.

**Fosved application:**
- Telegram bot uses Telegram conventions (slash commands, inline buttons, file uploads).
- Miniapp navigation: bottom tab bar (mobile-familiar), not invented pattern.
- Settings screen looks like other settings screens, not a clever reimagination.

**Caveat:** sometimes innovation is the value. Pick deviations consciously, document why.

### Doherty Threshold — productivity

**Formulation:** when user and computer interact below 400ms latency, productivity soars.

**Design implications:**
- **Response time matters** — every interaction <400ms preferred.
- **Immediate feedback** — even loading state instantly visible.
- **Optimistic UI** — show success before server confirms, rollback if fails.

**Fosved application:**
- Yana acknowledges command receipt within 400ms ("⏳ Processing...") even if work takes longer.
- Miniapp form submissions: optimistic update, then sync.
- Search-as-you-type: results updated within 400ms (debounced).

### Serial Position Effect — list memory

**Formulation:** users remember first and last items in a list better than middle.

**Design implications:**
- **Most important items at start or end** of lists, navigation.
- **Important nav items: leftmost or rightmost** (in LTR cultures).
- **Middle of long lists** — easy to overlook. Use grouping/scanning aids.

**Fosved application:**
- Quick action buttons in miniapp drive: most-used first and last.
- OSP scenarios list: most-likely first, least-likely last; middle ones less differentiated.
- README sections: hook at start, quick-start near top, deep details middle, contact/license end.

### Tesler's Law — complexity conservation

**Formulation:** every system has irreducible complexity. You can shift it but not eliminate it.

**Implication:** either user deals with complexity or designer/dev does. Hidden complexity in implementation > exposed complexity in UI.

**Design implications:**
- **Sensible defaults** for everything. User can override but doesn't have to.
- **Auto-detect** what can be (timezone, language, format).
- **Hide rarely-used options** behind "Advanced".

**Fosved application:**
- DevTask creation: estimate auto-suggested from similar tasks, owner can override.
- Architecture decisions: template fields auto-filled where possible.
- Visual department: template fields auto-mapped from source data, only edge cases need manual input.

### Goal Gradient Effect — progress motivation

**Formulation:** motivation increases as users approach a goal.

**Design implications:**
- **Show progress** for multi-step flows.
- **"X% complete" or "Step N of M"** — keeps users engaged.
- **Tasks near completion** — make easy to finish (don't add friction at the end).

**Fosved application:**
- Multi-phase migration plans (like DDQ integration) show phase X of N.
- Onboarding flows: progress bar with named steps.
- DevTask completion: show steps remaining.

### Pareto Principle (80/20)

**Formulation:** ~80% of effects come from ~20% of causes.

**Design implications:**
- **Most users use few features.** Optimize for them.
- **Power features behind progressive disclosure** for the 20%.
- **Spend design effort on the 80% paths**.

**Fosved application:**
- Yana defaults: 80% of requests routed via 5 specialists. Others reachable but secondary.
- OSP commands: `/analyze` is 80% case; `/quickanalyze`, `/deepanalyze` for the 20%.
- Miniapp: drive-screen most used, profile screen rarely — design effort allocated accordingly.

### Peak-End Rule

**Formulation:** people judge experiences by their peak (best/worst moment) and end, not average.

**Design implications:**
- **End strong** — last interaction shapes memory.
- **Avoid bad peaks** — one really bad moment ruins overall impression.
- **Confirmation/success screens** — make them positive moments.

**Fosved application:**
- Task completion: clear celebratory confirmation, not just silent end.
- Error recovery: make recovery experience pleasant (low-friction retry, helpful message).
- Long operations: end with summary "Done: 47 records processed, 3 minutes elapsed".

## Applying the laws — workflow

When reviewing a design:

1. **List actions** users take on this screen.
2. **Apply Fitts** — each interactive element appropriate size/distance?
3. **Apply Hick** — number of choices reasonable?
4. **Apply Miller** — info chunked or overwhelming?
5. **Check Von Restorff** — does primary stand out enough?
6. **Check Jakob** — does this look like users expect?
7. **Visual consistency** for aesthetic-usability — yes?
8. **Loading feedback** within Doherty threshold?
9. **Important info** at start/end per serial position?

Document compliance in `DesignDecision` records. If violating a law: document WHY (sometimes deviation is intentional).

## Cognitive load assessment

For each critical screen, score:
- **Intrinsic load** (task complexity itself) — inherent, can't reduce
- **Extraneous load** (UI complexity unrelated to task) — REDUCE THIS
- **Germane load** (mental effort to learn) — useful for first-time users

Aim: minimize extraneous, manage intrinsic, support germane through education.

Heavy cognitive load symptoms in users:
- Hesitation (>2s deciding next action)
- Re-reading text
- Asking for help
- Errors on first attempt

Each is a design failure to investigate.

## Anti-patterns

- **Applying laws as rules.** Laws are observations, not commandments. Sometimes context overrides. Use judgment.
- **Hick's Law as paralysis.** "Only 3 options ever." Sometimes 10 grouped options work fine.
- **Citing laws to justify bad design.** "It's Fitts's Law" — but the button is in a bad place. Law doesn't excuse.
- **Ignoring laws because they're "psychology not design".** They're measurable design constraints.
- **Applying without measurement.** Did the design improve? Test with users or analytics.
- **Designer's intuition vs law conflict.** Usually law wins. Designer's intuition is biased by exposure.

## Worked example — Yana command menu

Before:
- 30 commands listed alphabetically on `/help`
- Equal visual weight to all
- No grouping

Applying laws:

**Hick's Law:** 30 choices = ~5 units decision time. Too many. → Group by specialist (5-7 groups of 3-7 commands each).

**Miller's Law:** Group sizes 3-7 = within working memory. → Yes.

**Von Restorff:** Most-used commands highlighted (gold). Others standard. → Yes.

**Serial Position:** Most important groups (OSP, Expert) at top and bottom. Specialist tools middle. → Adjusted.

**Aesthetic-Usability:** Polish: consistent emoji per group, consistent format, monospace for command names. → Yes.

**Pareto:** 80% of usage probably 5 commands. Those prominently visible. Rest in `/help all`. → Yes.

Result: 30 commands now in 6 named groups, prominent commands stand out, full list still accessible. Faster decisions, less cognitive load.

## Integration

- Promoted to Tier 1 — applies to every design
- `wireframe-production` references during structure
- `interaction-states` informed by Fitts (target size)
- `responsive-design` informed by Fitts (touch targets)
- `dev-handoff-specs` includes cognitive load assessment
- `heuristic-evaluation` includes laws-based checks
