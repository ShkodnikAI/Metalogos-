# The Red-Team Protocol — the independent attack pass (№961)

> The trust the tests earn is the trust of their AUTHOR. The audit of
> 12.11 proved the failure mode: the label-loss through a pattern
> (№312's shape) lived for two months behind green gates because the
> same executor wrote the code, the tests and the facts. This protocol
> makes the ATTACK a separate, named act — performed by an executor who
> did not write the thing under attack and cannot merge the fix.

## 1. When a pass runs

- **Every security-class PR** — a PR whose title carries the `[security]`
  class marker or that carries the `security` label — must attach a
  red-team pass to its description (`red-team: <report link>`) before it
  merges. The CI gate (`scripts/ci/red_team_gate.py`, the
  `red-team-gate` blocking job) enforces the field mechanically.
- **Every release candidate** — the release cut (rc or stable) gets a
  full pass against the CANDIDATE BINARY; the release notes carry the
  line `Red team: <date, result>` (the release-checklist step).
- **On demand** — the owner or the dispatcher can order a pass on any
  surface; the same artifact form applies.

## 2. The adversary's posture (the independence contract)

- The adversary is a **separate agent/session** that did NOT write the
  code under attack, has NO access to the merge rights, and does NOT
  read the fixing naryad's text, the implementation details, the ADRs or
  the tests. **Black-box only**: the CANDIDATE BINARY and the promise
  sheet (the README's security claims, extracted verbatim —
  `pocs/PROMISES.md` of the pass report).
- The adversary works in its own directory; the probes are written
  against the BINARY (`mlog check` / `mlog run`), never against the
  sources. Network access: none (the probes use non-routable
  placeholders).
- The adversary's report is the artifact of record; the executor of the
  fix may NOT edit it — a finding is answered by code or by a named
  boundary, never by rewording the attack.

## 3. The report form (the artifact)

One directory per pass: `docs/audits/red-team-<date>/`:
- `report.md` — the executive summary, the probe table (probe → the
  promise attacked → the check verdict → the run verdict → BYPASS or
  CAUGHT), and the full evidence for every BYPASS (the probe source +
  the observed output verbatim). The verdict discipline: a BYPASS means
  the promise TEXT is broken by OBSERVED behavior; an inconclusive probe
  is not a bypass.
- `pocs/` — every probe program, named `pNN_<shape>.mlog`, plus the
  raw check transcripts and the promise sheet. The form is the
  `audit_2026-11-12/pocs` precedent.

## 4. What a finding does (the №956 п.3 rule, applied)

- **A BYPASS that matches NO documented row** → a leak-suite negative or
  a fuzzer form lands in the SAME PR that receives the report (the
  №956 п.3 rule: a form found by an audit is fixed in the PR that
  carries it) — or the PR does not merge.
- **A BYPASS that matches an OPEN, owner-held row of
  `docs/threat-model.md`** (a measured decision boundary, e.g. the
  B13 row — the flow output's integrity half, ADR-0154 §13 Proposed) →
  the report records the probe AGAINST that row; no pin flips, because
  the behavior is the owner's named decision, not an omission. The row's
  naryad receives the probe as extra evidence.
- **A CAUGHT probe** is still valuable — it is a free conformance test;
  the report keeps it; a particularly sharp CAUGHT shape can be adopted
  into the leak suite's positive set by the maintainer's choice.

## 5. The pass record of №961 (the first pass — the done-when)

The first pass ran on the **v0.40.0 candidate** (the released binary,
SHA256 `bedb6d29…`, the №950–№955 content), executed by an independent
adversary session on 2026-11-16: 46 probes, **1 BYPASS, 38 CAUGHT,
5 inconclusive** — and the bypass (`p29_llm_flowout`) independently
rediscovered the MEASURED owner-held boundary **B13** (the flow output's
integrity half — the confidentiality axis is gated by №965, the
integrity half is №966's measured package awaiting the owner's form
choice; `p33_cron_expired` documents the ttl=0 no-window convention).
The report and the probes: [red-team-2026-11-16/](red-team-2026-11-16/).
No undocumented bypass was found; the gate, the label lattice and the
sanitizers held every conduit probed (structs, lists, loops, match
branches, pattern chains, flows, string transforms, concatenation after
sanitization).
