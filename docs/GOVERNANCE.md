# Metalogos Governance

This document describes the governance model for the Metalogos project.

## Overview

Metalogos is an open-source project created and primarily maintained by a solo developer (ShkodnikAI). As the project grows, we are committed to evolving toward a community-driven governance model.

## Current Structure (Phase 1: BDFL)

### Benevolent Dictator for Life (BDFL)

- **ShkodnikAI** — Project founder, lead developer, and final decision-maker
- Responsible for: architecture decisions, release management, security policy, funding

### Rationale

During the early stages of the project (pre-1.0), a BDFL model ensures:
- Rapid decision-making
- Consistent vision and direction
- Clear accountability
- Efficient resource allocation

## Decision-Making Process

### Types of Decisions

| Type | Description | Decision Maker |
|------|-------------|----------------|
| **Technical** | Language design, compiler architecture, API changes | BDFL (with community input) |
| **Security** | Security policy, vulnerability handling, CVEs | BDFL |
| **Financial** | Funding, sponsorship, budget | BDFL |
| **Community** | Code of Conduct enforcement, contributor recognition | BDFL + Community moderators |
| **Release** | Versioning, release schedule, feature freeze | BDFL |

### Community Input

For all significant decisions:
1. Open a GitHub Discussion with the `decision` label
2. Allow at least 7 days for community feedback
3. BDFL makes the final decision, weighing community input
4. Decision is documented in the relevant issue/PR

### Breaking Changes

Breaking changes require:
1. RFC (Request for Comments) in Discussions
2. Minimum 14-day comment period
3. BDFL approval
4. Documentation update
5. Migration guide (if applicable)

## Future Governance (Phase 2: Core Team)

As the project matures and gains active contributors, we will transition to a **Core Team** model:

### Core Team

- 3–7 members with merge rights
- Elected by community vote (annually)
- Responsible for day-to-day maintenance and review

### BDFL Role Evolution

- BDFL retains veto power on architectural decisions
- BDFL focuses on long-term vision and strategy
- BDFL can step down or be replaced by Core Team vote (2/3 majority)

### Transition Criteria

We will initiate Phase 2 when:
- 5+ active contributors with sustained contributions (6+ months)
- 3+ external maintainers with deep codebase knowledge
- Stable funding (sponsorship covering >50% of development time)
- Community vote in favor (simple majority)

## Communication Channels

- **GitHub Issues** — Bug reports, feature requests
- **GitHub Discussions** — Architecture debates, RFCs, general questions
- **Security** — security@metalogos.dev (private)
- **Conduct** — conduct@metalogos.dev (private)

## Conflict Resolution

1. Discuss in the relevant GitHub thread
2. If unresolved, escalate to BDFL
3. For conduct issues, see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
4. For security issues, see [SECURITY.md](SECURITY.md)

## Financial Transparency

All financial activities are publicly documented:
- GitHub Sponsors dashboard (public)
- Open Collective budget (public)

## License

This governance document is licensed under CC-BY-4.0.

---

*Last updated: 2026-08-09*

## Institutional Phase-2 Frame: the Neutral Structure (the 8H-1 / C13б subphase)

Added by the wave-83 institutional contour (ADR-0219,
docs/adr/0219-institutional-contour-legal-shield.md). This section
APPENDS to the governance document; the Phase-1 text above is
unchanged byte-for-byte (the 1:1 migration discipline).

### The neutral-structure criteria

If and when the project's assets (the repository, the domains, the
release infrastructure) are moved into a holding structure, the
structure must satisfy ALL of the following criteria — the criteria
are the frame, not a structure:

1. **Jurisdictional neutrality** — the structure's jurisdiction is
   chosen so the project's legal existence does not concentrate in a
   single high-risk point tied to one person's residence or one
   regulator's mood; the criteria here configure the constraints and
   name no jurisdiction («конфигурация ограничений, не юридический
   совет» — the C10 frame, ADR-0209 §5).
2. **Protection from a single point of jurisdictional pressure** — no
   single court or regulator action in one jurisdiction can seize or
   stall the whole contour (the documents, the name, the release
   chain) without visible recourse; the assets and the decision
   authority must not share one failure domain.
3. **Continuity of asset ownership** — the structure has its own
   succession path that maps onto the repository's succession
   protocol ([docs/SUCCESSION.md](SUCCESSION.md) — the triggers, the
   authority chain, the append-only journal) instead of depending on
   one natural person.

The structure itself — the incorporation, the bank, the filings — is
an OWNER ACT outside this repository; it is never opened as a naryad
and never recorded here (see the honest limits below).

### Cross-consistency with TRADEMARK.md and SUCCESSION.md

- [TRADEMARK.md](../TRADEMARK.md) — the name/logo policy: a holding
  structure does not become a trademark licensee by default; the
  structure's documents must not conflict with the trademark policy's
  neutral-use rules.
- [docs/SUCCESSION.md](SUCCESSION.md) — the succession protocol: the
  structure's own succession path must CONSUME it (the triggers, the
  authority chain by reference, the journal), never duplicate or
  restate it.
- The Phase-1 text of this document (above) stays authoritative for
  the decision-making process; this frame adds the institutional
  boundary and does not alter any Phase-1 rule.

### Honest limits — the owner's acts (the repo ↔ acts boundary)

The repository carries DOCUMENTS, not legal acts. The following are
the owner's acts and live OUTSIDE the repository (ADR-0219 §5;
R38(9)): the trademark filing, the neutral-structure incorporation,
the banking, and the partner contacts (the audit firms → C1, the
insurers → C2 — the partner track is the owner's time). Nothing in
this section is legal advice; the machine gates (the gate_044
institutional roster) verify the PRESENCE and STRUCTURE of these
documents and their cross-consistency — not their legal force.
