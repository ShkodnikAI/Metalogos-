---
name: adjacent-field-cross-check
description: Tests project claims through adjacent fields whose principles bear on the project. Physics test on engineering claims; engineering test on physics claims; manufacturing test on lab demos; economics test on technical solutions. Adjacent-field perspective often reveals issues invisible within the project's nominal domain because the experts there share the same blind spots. Cross-checks surface failures that single-domain reviewers consistently miss.
---

# Adjacent Field Cross-Check — Looking From The Side

Experts in a field share assumptions invisible to them. The fusion physicist may not know what's hard in materials science. The materials scientist may not know what's hard in plasma physics. Both think their domain is the bottleneck. The project's actual bottleneck might be invisible to both because it's at the boundary.

This skill is the discipline of asking how the project looks from adjacent fields. The questions adjacent-field experts would ask often reveal what within-field experts miss.

## Prerequisites

- `rapid-domain-immersion` complete for primary field
- Identification of relevant adjacent fields
- Sufficient familiarity with adjacent fields to identify their concerns

## Core principle

> Within any technical field, experts share blind spots invisible to themselves. Adjacent fields whose principles bear on the project see different things. Asking "how does this look from materials science?" or "how does this look from manufacturing?" reveals issues the primary-field experts don't notice. The skill is identifying which adjacent fields apply and what they would see.

## Common adjacent-field perspectives

### Physics ↔ Engineering

**Physics on engineering claims:**
- "The engineering claim assumes parameters that violate <specific physics constraint>"
- "The energy balance doesn't add up under realistic operating conditions"
- "Heat dissipation requires <specific physics> not addressed"

**Engineering on physics claims:**
- "The physics works in principle, but the engineering tolerances required are unprecedented"
- "Manufacturing the geometry to required precision is order of magnitude harder than stated"
- "Materials don't exist with required properties at scale"

### Lab ↔ Manufacturing

**Lab on manufacturing claims:**
- "Reproducibility at lab scale is X; at manufacturing scale Y is required"
- "Quality variance grows substantially with batch size"

**Manufacturing on lab claims:**
- "What works in single-shot lab doesn't scale to continuous"
- "Cost structure assumes manufacturing capabilities that don't exist"
- "Yield assumptions are unrealistic for first-of-kind production"

### Theory ↔ Application

**Theory on application claims:**
- "Application assumes performance levels not achievable theoretically"
- "Generalization beyond tested conditions is unjustified"

**Application on theory claims:**
- "Theory ignores deployment-relevant variations (temperature, humidity, vibration)"
- "Real-world inputs are not the cleaned datasets used in proof"

### Biology ↔ Chemistry ↔ Physics

For biotech and pharma:
- **Biology on chemistry claims:** "The molecule is technically achievable but cellular uptake/excretion isn't addressed"
- **Chemistry on biology claims:** "The biological mechanism requires concentrations the chemistry can't achieve"
- **Physics on either:** "The thermodynamic constraint is being violated"

### Economics ↔ Technology

- **Economics on technology:** "Technology works but cost structure makes deployment infeasible"
- **Technology on economics:** "Cost projection assumes technology improvements that aren't grounded"

### Regulatory ↔ Technology

- **Regulatory on technology:** "Technical approach faces regulatory pathway not addressed"
- **Technology on regulatory:** "Regulatory plan assumes technology meeting standards it doesn't"

### Materials ↔ Energy/Systems

- **Materials on systems:** "System operation depends on materials with properties not yet achieved"
- **Energy on materials:** "Materials work in static conditions; energy flux destroys them"

## The procedure

### Step 1 — Identify adjacent fields relevant to project

For the specific project, which adjacent fields' principles bear?

For fusion: physics (plasma), engineering (cryogenics, magnets), materials science (first wall), nuclear chemistry (tritium), economics (utility infrastructure), regulatory (nuclear), supply chain (rare materials).

For mRNA cancer therapy: biology (immunology), chemistry (nucleic acid stability), pharma (delivery), regulatory (FDA), manufacturing (cold chain), economics (per-patient cost).

For an AI startup: computer science (algorithms), hardware (compute requirements), data (acquisition and curation), energy (training cost), regulatory (privacy, safety), economics (unit economics).

Aim for 4-7 adjacent fields per major project. Less misses key perspectives; more dilutes attention.

### Step 2 — For each adjacent field, generate critical questions

What would an expert from that adjacent field ask about this project?

Not generic questions but field-specific ones revealing assumptions the primary-field experts may not see.

### Step 3 — Test project against each adjacent perspective

For each adjacent field's questions, work through whether project addresses them. Common findings:
- Project addresses some adjacent perspectives well
- Project ignores others
- Project's assumptions violate adjacent-field knowledge

### Step 4 — Identify cross-field gaps

The decisive gaps are where:
- Adjacent field would identify a critical issue
- Project doesn't address it
- Within-field experts may not notice it

These are particularly valuable findings because they're often invisible to traditional due diligence (which usually operates within field).

### Step 5 — Generate adjacent-field combat questions

Format: "I was talking to a [adjacent field expert] and they suggested asking about [specific issue]. How do you address this?"

This framing has tactical advantage: positions question as coming from credible adjacent expert, hard to dismiss.

### Step 6 — Output cross-check report

Structured output by adjacent field with critical gaps and meeting questions.

## Worked example — fusion startup adjacent fields

### Adjacent field: Materials science

**Materials questions about FusionCorp:**
- "What materials work for first wall under 14 MeV neutron flux for 5+ years?"
- "How do you address swelling, embrittlement, transmutation in structural materials?"
- "Tritium retention in materials — controlled?"

**Project's address:** generic mention of "advanced materials development" without specifics.

**Cross-field gap:** materials science views first wall as 10-15 year challenge; commercial timeline assumes solved without specifying how.

### Adjacent field: Cryogenic engineering

**Cryogenic engineering questions:**
- "HTS magnets need 20-50 Kelvin operation; what's your cooling system architecture?"
- "Failure modes of HTS at scale — quench protection?"
- "Thermal cycling during operation/maintenance — material fatigue?"

**Project's address:** mention of HTS magnets without cryogenic system details.

**Cross-field gap:** cryogenic system is half the engineering challenge of HTS magnet integration; not addressed.

### Adjacent field: Nuclear engineering

**Nuclear engineering questions:**
- "Tritium handling and inventory management?"
- "Decommissioning plan for activated structures?"
- "Emergency response for tritium release?"
- "Regulatory pathway as nuclear-equivalent facility?"

**Project's address:** acknowledged "nuclear-like" regulation expected but no specific plan.

**Cross-field gap:** nuclear engineering practices are deeply established; treating fusion as "different from fission" without engaging with nuclear engineering norms is common error.

### Adjacent field: Utility/grid integration

**Utility perspective questions:**
- "Plant load factor — fusion can't ramp like gas turbines, are you baseload?"
- "Grid integration — what's the response time to demand changes?"
- "Coupling with energy storage if not load-following?"

**Project's address:** generic mention of "baseload power."

**Cross-field gap:** utility integration is real challenge; baseload market is shrinking with grid evolution toward renewables + storage.

### Adjacent field: Economics of energy infrastructure

**Energy economics questions:**
- "Levelized cost of energy (LCOE) projection compared to alternatives?"
- "Capital recovery factor over plant lifetime?"
- "Fuel cycle costs (tritium production)?"
- "Decommissioning provisioning?"

**Project's address:** $3B/GW capex without LCOE breakdown.

**Cross-field gap:** energy economics evaluates against LCOE; capex alone is incomplete metric.

### Cross-field combat questions:

1. "I was talking to a materials scientist who suggested asking about first wall — what materials work for 14 MeV neutron flux for 5+ years, and what's your specific development partnership?"

2. "From a cryogenic engineering perspective, HTS magnet quench protection at scale is a known challenge — how do you handle quench events in commercial operation?"

3. "Nuclear engineering perspective: what's your tritium inventory management and emergency response plan?"

4. "From utility integration perspective, fusion plants can't ramp like gas turbines — what's your role in a grid increasingly dominated by intermittent renewables and storage?"

5. "On energy economics — what's your projected LCOE not just capex, including fuel cycle and decommissioning?"

These questions are often more penetrating than within-field questions because they address what within-field experts may overlook.

## Anti-patterns

- **Adjacent fields too distant.** Asking pure pure mathematics about fusion adds nothing. Pick fields whose principles actually bear on the project.
- **Generic "what about other perspectives."** Specific field-grounded questions are what works.
- **Ignoring field expert dependencies.** When their team has experts in adjacent fields, those concerns may be addressed. Check team.
- **Overweighting one adjacent field.** Each field has biased lens. Multiple cross-checks balance.
- **Tactical framing failure.** The "I was talking to a [field expert]" framing is powerful; using it credibly requires having actually consulted (in this case, having simulated). Don't fabricate specific people.

## Output template

```
─── ADJACENT FIELD CROSS-CHECK ───

Project: <identifier>
Adjacent fields identified: <list>

[For each adjacent field:]

ADJACENT FIELD: <name>
Critical questions: <list>
Project's address: <description>
Cross-field gap: <specific issue>

[Repeat for 4-7 fields]

CROSS-FIELD COMBAT QUESTIONS:
1. <Question with adjacent-field framing>
2. [more]
[3-5 typical]

GAPS WITHIN-FIELD MIGHT MISS:
- <Gap 1>: from <adjacent field> perspective
- <Gap 2>: similar
```

## Integration with Expert protocol

Tier 2 — invoked for:
- Multi-disciplinary projects (most serious technical projects qualify)
- High-stakes evaluations
- When primary field knowledge is limited (rely more on adjacent perspectives)

Output integrates into combat questions.

Stored in ExpertBriefing under `adjacentFieldCrossCheck`.

This skill produces some of the most penetrating findings of Expert work. The questions adjacent fields ask are often the questions that, in retrospect, predicted project failures.
