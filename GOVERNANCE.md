# Metalogos Governance

This document describes the governance model for the Metalogos project.

## Overview

Metalogos is an open-source project created and primarily maintained by a solo developer (ShkodnikAI). As the project grows, we are committed to evolving toward a community-driven governance model.

## Current Structure (Phase 1: BDFL)

### Benevolent Dictator for Life (BDFL)

- **ShkodnikAI** — Project founder, lead developer, and final decision-maker
- Responsible for: architecture decisions, release management, security policy, grant applications

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
| **Financial** | Grant applications, sponsorship, budget | BDFL |
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
- Stable funding (grants or sponsorship covering >50% of development time)
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
- Grant applications and reports (public, where permitted by funders)

## License

This governance document is licensed under CC-BY-4.0.

---

*Last updated: 2026-08-09*
