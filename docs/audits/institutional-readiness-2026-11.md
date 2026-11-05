# Institutional-readiness report: the verified 12-item roster and the repo ↔ owner-acts boundary (the 8H-1 subphase)

**Naryad №919 (gh#1833, wave 83 — W-L1), class [governance][docs][audit].**
The readiness artifact of the institutional-contour subphase (8H-1;
C13б — the legal shield: the trademark + the succession + the neutral
structure — BEFORE the first paying pilot). Every verdict below is
regenerable by the machine (`scripts/gen_metrics.py --check`, the
gate_044 roster) or readable verbatim in the named live source — no
prose verdicts.

**Report date:** 2026-11-05. **The tree:** main `e15a048ea` (the
wave-83 merge chain). **The record:** ADR-0219 (the №916 subphase
record — the fact definition, the roster, the contour boundary).

## 1. What INSTITUTIONAL_READY counts — the machine verdict on this tree

```
OK: the №918 floor holds — INSTITUTIONAL_READY 12/12 (gate_044 floor
12, only-up) — roster: 12 institutional items over the live carriers
```

The roster (`scripts/ci/gate_044_goals.txt`, the №862/№894 form — the
count is CHECKED, not declared; a row counts only when its needle (the
section heading) AND its sub literal stand in the named live file, and
the ADR-0219 definition literal — the composition site — must stand):

| # | Item | Carrier | The machine anchor (needle + sub) |
|---|------|---------|-----------------------------------|
| 1 | the scope of the name and the logo | TRADEMARK.md | `## 1. Scope — the name and the logo` + `This policy covers` |
| 2 | the permitted use | TRADEMARK.md | `## 2. Permitted use` + `No prior permission is needed for` |
| 3 | the forbidden use + the anti-endorsement rule | TRADEMARK.md | `## 3. Forbidden use and the anti-endorsement rule` + `The anti-endorsement rule` |
| 4 | the permission-request channel | TRADEMARK.md | `## 4. The permission-request channel` + `trademark request:` |
| 5 | the activation triggers | docs/SUCCESSION.md | `## 1. Activation triggers` + `T1 — unavailability` |
| 6 | the authority chain (by reference) | docs/SUCCESSION.md | `## 2. The authority chain (consumed by reference — never duplicated)` + `maintainers.md` |
| 7 | the recovery matrix | docs/SUCCESSION.md | `## 3. The recovery matrix` + `The release signing keys` |
| 8 | the quorum / decision continuity | docs/SUCCESSION.md | `## 4. Quorum and decision continuity` + `does not invent a council` |
| 9 | the append-only activation journal | docs/SUCCESSION.md | `## 5. The activation journal (append-only)` + `No activations recorded` |
| 10 | the Phase-2 neutral-structure frame | docs/GOVERNANCE.md | `### The neutral-structure criteria` + `Jurisdictional neutrality` |
| 11 | the cross-consistency | docs/GOVERNANCE.md | `### Cross-consistency with TRADEMARK.md and SUCCESSION.md` + `../TRADEMARK.md` |
| 12 | the owner-acts boundary | docs/GOVERNANCE.md | `### Honest limits — the owner's acts (the repo ↔ acts boundary)` + `R38(9)` |

The tamper traps (`gen_metrics.py --tamper-test`, traps №46–№50) prove
each dispatch class red: the removed section, the emptied section, the
broken cross-reference, the dead ADR-0219 definition, the lowered
floor. The floor is 12 only-up — the wave-83 dispatch constant
(gh#1834); a lowering is an owner transaction, never a gate edit.

## 2. The three shield parts as they stand — audited from the sources

1. **The trademark policy (TRADEMARK.md).** The neutral-use rules for
   the name and the logo: the scope (any casing, the compound marks,
   the badge/wordmark assets; the `.mlog` extension and the grammar
   are code, not marks), the permitted use (the honest reference, the
   truthful compatibility statements, the non-commercial mention, the
   unmodified redistribution, the nominative use), the forbidden use
   with the anti-endorsement rule (a modified build must be explicitly
   marked as modified and unaffiliated; the absence of a written
   permission means NO endorsement), and the request channel — a
   GitHub issue titled `trademark request:` on the repository's own
   tracker (the decision follows docs/GOVERNANCE.md: the BDFL decides,
   the decision is recorded in the issue). No invented addresses: the
   policy binds to the surfaces that exist today.
2. **The succession protocol (docs/SUCCESSION.md).** The triggers
   T1/T2/T3 (the >30-day unavailability with two recorded attempts;
   the voluntary transfer; the loss of access) — a trigger is
   RECORDED, not assumed. The authority chain consumes
   docs/maintainers.md (№471) and .github/CODEOWNERS BY REFERENCE —
   the successor steps INTO the existing chain, the FROZEN perimeter
   does not grow. The recovery matrix covers the repo admin (the org
   transfer + the CODEOWNERS handle update), the domains (the
   registrar of record — an owner act), the release keys (NO
   human-held signing key — the Sigstore keyless chain,
   .github/workflows/release.yml; a successor re-runs the release
   workflow, the registry pins re-verify by digest — ADR-0218), and
   the decision authority (the quorum read from the LIVE holders —
   today the single-holder fact, the lane №774 verified-wait
   (gh#1437)). The activation journal is append-only and EMPTY by
   design — no trigger has ever occurred; the journal records acts
   and never creates authority.
3. **The neutral-structure frame (docs/GOVERNANCE.md, the appended
   Phase-2 section).** Three criteria any holding structure must
   satisfy: jurisdictional neutrality, protection from a single point
   of jurisdictional pressure, continuity of asset ownership (the
   structure's own succession maps onto docs/SUCCESSION.md). The
   criteria are the frame — not a structure, not a jurisdiction
   choice, and not legal advice (the C10 verbatim: «конфигурация
   ограничений, не юридический совет»). The Phase-1 BDFL text is
   byte-identical (the merge diff was a pure append: +60/-0).

## 3. The boundary: the repo carries documents — the owner performs acts

The machine verifies the PRESENCE and STRUCTURE of the policies and
their integrity (the cross-references, the append-only journal) — NOT
their legal force (ADR-0219 §5). The following acts are named by the
plan (R38(9)) and live OUTSIDE the repository; none is opened as a
naryad, none has a stub record here:

| The owner's act | Status | What the repo carries |
|---|---|---|
| The trademark FILING (the application to the trademark office) | the owner's act, outside the repo | TRADEMARK.md — the policy the filing would protect |
| The neutral-structure INCORPORATION | the owner's act, outside the repo | the GOVERNANCE.md Phase-2 criteria frame the structure must satisfy |
| The banking | the owner's act, outside the repo | nothing (no account, no bank references in the tree) |
| The partner contacts (the audit firms → C1, the insurers → C2) | the owner's act, outside the repo | the registry STRUCTURE below (§4) — no records |
| The renaming №503 (Metalogos- → Metalogos) | the owner's act, strictly last | not started in this wave |

Reading any machine verdict as «the trademark is filed» or «the
structure exists» would be a lie — the gate counts DOCUMENTS.

## 4. The partner-contact registry structure (columns + criteria, no records)

The partner track is the owner's time (R38(9)); the repo fixes the
STRUCTURE of the contact registry — the columns and the readiness
criteria — while the records themselves are owner acts. The structure
(the columns a future record fills):

- **The partner class** — the auditors (C1) / the insurers (C2) /
  the other institutional counterparties;
- **The target capability** — what the partner is contacted FOR (the
  C1: the external audit of the C9-class guarantees; the C2: the
  coverage of the institutional-contour risks);
- **The readiness criterion** — the machine-verifiable state that must
  hold BEFORE the contact (for C1: the subphase's own report is
  accepted and the machine facts hold their floors; for C2: the
  succession documents + the recovery matrix are in the tree and the
  owner's acts (§3) have progressed far enough to be insurable);
- **The owner-act reference** — the contact itself, its date and its
  outcome live in the owner's records, never in the repo.

**No records exist yet** — the registry structure is empty by design;
a stub row here would be a lie about a contact that has not happened.

## 5. What the succession coverage does NOT close (the honest limits)

- The succession matrix + the Sigstore keyless chain + the
  CODEOWNERS perimeter reduce the bus-factor exposure of the
  TECHNICAL assets (the repository, the release chain, the registry
  pins). They do NOT create a legal person, do NOT file the
  trademark, and do NOT hold a bank account — the §3 acts stay the
  owner's.
- The single-holder fact stands: the second maintainer (№774,
  gh#1437) is verified-wait — the authority chain references one
  holder today. The succession documents prepare the path; they have
  never been walked (the journal is empty by design).
- The gate reads the tree, not the world: a policy can be altered by
  a normal PR — the tamper traps catch the SILENT structural damage
  (a removed section, an emptied section, a broken reference, a dead
  definition), not the legal validity of any text.
- The external institutional audit is not procured by this wave (the
  R38(3) owner gate with the reserved budget stands separately; the
  C1 partner track is the owner's time).

## 6. The docs sync landed in this naryad

- `docs/limitations.md` — the institutional-contour row: what the
  machine fact covers (the presence + the structure + the integrity),
  what it does not (the legal force; the owner's acts), the
  bus-factor risk №1 and its partial coverage.
- `docs/threat-model.md` — the institutional row: what the shield
  already catches (the silent structural damage — the trap classes),
  what remains (the owner's acts, the single-holder fact, the legal
  force outside the machine's claims).
