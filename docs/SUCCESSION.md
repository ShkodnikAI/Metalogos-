# Succession — the written authority for a bus-factor event

The succession protocol of the institutional contour (ADR-0219 — the
roster items 5–9; the 8H-1 / C13б subphase). The project's bus factor
is 1 (docs/maintainers.md, №471) — this document prepares the recovery
path so a bus-factor event meets a POLICY, not improvisation.

The status fact: the second maintainer (the lane №774, gh#1437) has
NOT joined — verified-wait. Today the single authority holder is the
BDFL/owner (@ShkodnikAI — docs/GOVERNANCE.md, .github/CODEOWNERS).
This document references that reality; it does not imagine a quorum
that does not exist.

## 1. Activation triggers

The protocol activates on any of:

- **T1 — unavailability:** the holder is unreachable for more than 30
  consecutive days on ALL live channels (the GitHub account, the
  issue tracker, the Discussions) — verified by two independent
  contact attempts at least 7 days apart, each recorded with its
  evidence link in the journal (§5);
- **T2 — voluntary transfer:** the holder announces the transfer in a
  GitHub issue on the repository; the announcement issue IS the
  trigger record;
- **T3 — loss of access:** the GitHub account is lost, compromised,
  or deleted, and the organization access is irrecoverable through
  GitHub's own recovery — verified in the journal with the evidence.

A trigger is RECORDED, not assumed: the journal (§5) carries the date,
the trigger class, and the evidence links. No other event activates
the protocol — the FROZEN perimeter of maintainers.md (№471) is not a
trigger, it is the thing the protocol protects.

## 2. The authority chain (consumed by reference — never duplicated)

The written authority lives in the existing documents; this protocol
consumes them BY REFERENCE:

- [docs/maintainers.md](maintainers.md) (№471) — the supply-chain veto
  perimeter and its FROZEN list; the onboarding and escalation rules;
- [.github/CODEOWNERS](../.github/CODEOWNERS) — the machine-read veto
  zone (the review requirement on the perimeter paths);
- [docs/GOVERNANCE.md](GOVERNANCE.md) — the BDFL role, the decision
  types, and the Phase-2 transition criteria.

A successor STEPS INTO the chain above as written — the succession act
updates the handles where the holder is named (the CODEOWNERS lines,
the maintainers.md references) and does NOT restate, edit, or grow the
perimeter itself: the №471 perimeter is FROZEN («it does not grow with
the codebase»), and growing it is an owner decision by definition
(maintainers.md, «Escalation»).

## 3. The recovery matrix

| Asset | Recovery path | The live reference |
|---|---|---|
| The repository (the GitHub org admin) | GitHub's org ownership transfer / the org recovery flow; the successor's handle is added to .github/CODEOWNERS and the maintainers.md references; the perimeter lines stay byte-identical | .github/CODEOWNERS; docs/maintainers.md |
| The domains | the registrar account of record transfers to the successor; the registrar of record is the owner's act (outside the repo — ADR-0219 §5) | the owner |
| The release signing keys | NO human-held signing key exists — the release chain is Sigstore keyless (.github/workflows/release.yml: a workflow-run attestation, not a person's key); a successor re-runs the release workflow from the repository; the registry pins re-verify the packages by digest | ADR-0218; .github/workflows/release.yml |
| The decision authority | the quorum of the LIVE holders per the authority chain (§2) at the moment of the act — today: the BDFL; after a succession: whatever the chain then states | docs/GOVERNANCE.md |

The recovery matrix names PATHS, not credentials: no secret, key, or
password lives in this repository (the secret-scanning gate enforces
the tree; the keyless release chain removes the human-key dependency).

## 4. Quorum and decision continuity

While the holder list has one entry (today's fact), the BDFL decides —
per docs/GOVERNANCE.md, unchanged. After a T2/T3 succession, decisions
continue under the SAME governance rules until the Phase-2 transition
criteria (docs/GOVERNANCE.md, «Future Governance») are met by their
own terms. This document does not invent a council, a vote, or a
quorum: the decision continuity is the authority chain (§2) read at
the moment of the act. An activation journal entry that claims a
quorum the chain does not state is INVALID — the journal records acts,
it does not create authority.

## 5. The activation journal (append-only)

The ledger of succession activations. Each entry records: the date,
the trigger class (T1/T2/T3), the act performed, and the evidence
links (the issue URLs). The journal is APPEND-ONLY: an entry is never
edited or removed — a correction is a NEW entry referencing the old
one. The journal lives in this section: every entry lands in the
repository history through a normal PR, and the git history of this
file is the tamper evidence.

**No activations recorded.** The journal is empty by design — no
succession trigger has ever occurred; the protocol prepares the path
and has never been walked.

Entry format (for the first activation, append below this line):

```
- YYYY-MM-DD | T<n> | <the act> | <the evidence links>
```
