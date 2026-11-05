# ADR-0220: Vertical Profiles — the Vertical Contour (Payments + the Public Sector) as a Typed, Fail-Closed, Digest-Pinned Configuration of Mandatory Contours (the 8H-2 / C13а Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-84 acceptance —
  the dispatch gh#1842 issued the subphase's naryads the same week; the
  wave acceptance fixes the ADR status as Accepted)
- Date: 2026-11-06
- Deciders: the OWNER (the R38 train — 0.47 = 8H-2 (C13а — the
  verticals: payments + the public sector, VERTICAL_PROFILES — the
  pre-last slot before the 1.0 assembly; R38(4)/(5): the vertical gate
  passed by the owner's directive «Заводи следующую волну нарядов
  согласно стратегического плана развития языка Металогос» of
  2026-11-06); the wave-84 dispatch gh#1842; the «Исполни наряды»
  directive of 2026-11-06)
- Naryad: №920 (issue #1839, Wave 84 — W-L2; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38 — the C13а line: the
  verticals payments + the public sector); the R62 GET-only fact-check
  over live main `be2d4c1f4` (2026-11-06)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.47 is this record's train slot, the
  pre-last before the 1.0 assembly),
  [ADR-0209](0209-jurisdiction-aware-policy-compilation.md) (the C10
  frame — the constraints-not-legal-advice boundary; the verbatim is
  quoted in §5 of this record and lives in ADR-0209 §5; the
  jurisdiction contour is mandatory contour (1)),
  [ADR-0205](0205-typed-approval-human-mandate.md) (the human-oversight
  contour — mandatory contour (2)),
  [ADR-0210](0210-deterministic-replay-forensic-ledger.md) (the
  decision-trace contour — mandatory contour (3)),
  [ADR-0217](0217-taint-saturation-formal-guarantees.md) (the taint
  floor — mandatory contour (4)),
  [ADR-0202](0202-model-passport-license-typed.md) (the license-typed
  model layer — mandatory contour (5); the №841 machine fact),
  the refusal vocabulary №385 (frozen — mandatory contour (6), the
  version-pin), [ADR-0218](0218-reproducible-build-signed-registry.md)
  (the digest-pin and the signed-package form — the [vertical]
  manifest table consumes the №910 [registry] shape; the package-path
  consumption inherits the №911 integrity gate),
  [ADR-0219](0219-institutional-contour-legal-shield.md) (the
  institutional contour — the legal shield BEFORE the first paying
  pilot; consumed by reference — the documents ≠ the owner's acts),
  G5 digest-pinning (gh#1527 — the trust anchor is an explicit pin in
  mlog.toml, the №828/№868 form), the C7 trigger (R38(8) — the first
  gov/bank pilot; the repro-readiness report №913: the trigger does
  NOT ignite from a profile)

## §1. Context — the train slot 0.47 demands verticals; the vertical surface in the tree is empty

The R38 train slot 0.47 = 8H-2 — the C13а line: the verticals
(payments + the public sector) — VERTICAL_PROFILES. The gate passed:
the wave-84 dispatch (gh#1842) is issued by the owner's directive, the
predecessor gate 0.46 is GREEN (INSTITUTIONAL_READY 12/12, gate_044;
the tamper traps №46–№50 alive; CI on `be2d4c1f4` — 53 success + 1
branch-freshness skip, 0 failed).

The R62 fact-check over live main `be2d4c1f4` (GET-only): the vertical
surface in the tree is EMPTY. «vertical» in the tree is only the
SVG layout vocabulary (src/builtins/svg/** — the gantt/tree_org/
layered diagrams) and the vertical-text prose in REFERENCE.md/
index.html; no vertical/industry-profile artifact exists (src/vertical*,
docs/vertical* — 0 hits). The name «profile» is TAKEN by the session
duty profile (src/profile.rs, №349/№281 — wake/duty): the vertical
artifact gets the separate name `vertical` (0 collisions).

The parents the vertical must stand on are LIVE in the code (the
Phase-8 contours exist as modules, not as promises): src/jurisdiction.rs
(№888 — JurisdictionMatrix, strictest-wins), src/approval.rs (№862 —
APPROVAL_COVERAGE), src/decision_trace.rs (№892 — REPLAY_MATCH),
src/model_passport.rs (№841 — LICENSED_FLOOR/PASSPORT_FRESH, the
license classes), metalogos-server/src/mcp_policy.rs (ADR-0196 — the
ToolPolicy), mlogpkg/ (№910/№911 — pack/keygen/sign/verify + the
fail-closed integrity gate), scripts/ci/fuzz_taint_triples_baseline.txt
(№905 — TAINT_SATURATION), the refusal vocabulary №385 (frozen).

The digest-pin pattern is established (G5, gh#1527): the trust anchor
is an EXPLICIT pin in mlog.toml — the compliance package №868 was
re-attested byte-for-byte at the wave-83 acceptance on exactly this
form. A vertical declaration is a configuration of the SAME class: it
must pin what it was attested against, or it refuses.

The machine-fact pattern is established (№841/№862/№894/№906/№912/№918):
a fact is a CHECKED count over a roster of live anchors with tamper
traps and an only-up floor — never a declaration. This record fixes
WHAT the vertical fact counts before any code lands (the №891/№916
form: the ADR fixes the definition, the naryad №921 builds the kernel
and the machine).

## §2. Decision — the fact is the 12-element vertical roster; two verticals; fail-closed consumption; no default attach

**VERTICAL_PROFILES := the count of CHECKED anchors (0–12) of the
vertical roster — 2 profiles (payments, public-sector) × 6 mandatory
contour anchors each (12 elements) — where every anchor is a
structural literal (the contour anchor in the profile's
mandatory_contours + the consumption sub-anchor) in the LIVE kernel
file src/vertical.rs, verified in CI; CHECKED, not declared.**

A vertical profile is a TYPED CONFIGURATION OF LIMITS AND MANDATORY
CONTOURS — never a certification, never legal advice, never a
regulator's acceptance (the C10 frame — the verbatim quoted in §5).
The profile carries:

1. **The typed identity** — `VerticalId::Payments | VerticalId::PublicSector`
   (two verticals by R38(5); medicine is BACKLOG until the PHI
   expertise exists — an owner decision, outside this wave).
2. **The six mandatory contours** (the Phase-8 contour set — a profile
   without the FULL six does not construct, the fail-closed
   constructor form №879/№888):
   - (1) **jurisdiction coverage** — the JurisdictionMatrix package
     (ADR-0209, strictest-wins);
   - (2) **human oversight / approval for irreversible** — the typed
     approval mandate (ADR-0205, the APPROVAL_COVERAGE contour);
   - (3) **decision-trace verify** — the replay/forensic ledger
     (ADR-0210, the REPLAY_MATCH contour);
   - (4) **the taint floor** — the taint saturation lane (ADR-0217,
     the fuzz_taint_triples baseline);
   - (5) **the license class of the backends** — the license-typed
     model layer (ADR-0202, the №841 LICENSED_FLOOR contour);
   - (6) **the refusal vocabulary** — the frozen №385 dictionary, the
     VERSION-PIN (a profile names the vocabulary generation it was
     attested against).
3. **The carrier digest pin** (the G5/№828 form): SHA-256 over the
   canonical serialization of the profile's own fields — the
   self-describing attestation anchor; a package declaring the
   vertical pins THIS digest in its manifest, and the consumption
   re-derives and compares byte-for-byte (the №868 re-attestation
   discipline).

The roster (the №862/№894/№918 form — id, carrier, anchor, sub-anchor;
the machine roster itself is gate_045, landed by №921):

| # | Profile | The contour anchor |
|---|---------|--------------------|
| 1 | payments | the jurisdiction-coverage contour |
| 2 | payments | the human-oversight/approval contour |
| 3 | payments | the decision-trace verify contour |
| 4 | payments | the taint-floor contour |
| 5 | payments | the license-class contour |
| 6 | payments | the refusal-vocabulary version-pin contour |
| 7 | public-sector | the jurisdiction-coverage contour |
| 8 | public-sector | the human-oversight/approval contour |
| 9 | public-sector | the decision-trace verify contour |
| 10 | public-sector | the taint-floor contour |
| 11 | public-sector | the license-class contour |
| 12 | public-sector | the refusal-vocabulary version-pin contour |

Every anchor is CHECKED, not declared — the №894 form. The floor is
**12 only-up** (the wave-84 dispatch gh#1842 constant; the №841 form:
the floor may rise, never fall — a lowering is an OWNER decision, the
№776 discipline).

**Fail-closed consumption (the №874/№845/№911 form).** A package
declaring a vertical does it EXPLICITLY in its manifest —
mlog.toml `[vertical]` with `id` + `digest` (the №910 [registry]
shape: the pin lives in the declaring package's manifest, never in a
side channel). The consumption contract:
- the declared `id` is not in the registry → refusal BEFORE any
  effect, the divergence NAMED («unknown vertical id»);
- the declared `digest` does not match the registry profile's
  carrier-digest pin → refusal BEFORE any effect, the divergence
  NAMED («vertical digest pin mismatch» — the package was attested
  against a different profile generation);
- a mandatory contour is not verifiable → refusal BEFORE any effect,
  the divergence NAMED.
A silent skip is impossible: there is no code path where a declared
vertical «passes loosely» — the verdict is all-or-refuse.

**No default attach (the negative control).** The absence of the
`[vertical]` declaration NEVER attaches a profile — not the payments
one, not «the closest match», not a default. A manifest without
`[vertical]` verifies exactly as it does today: ZERO behavioral
mutations on the undeclared path. The vertical contour is opt-in by
construction.

**The package path inherits №911.** The verdict rides AFTER the
integrity pass (the same ordering discipline: a tampered package
refuses as INTEGRITY first — the vertical verdict is a configuration
check over an already-integral package). The run-path/serve-path
composition (attaching the verdict to the interpreter's run loop and
the server startup) is OUTSIDE this wave — the honest limit is §5(4).

**The two verticals per R38(5).** `payments` (PSP/acquiring-grade
payment processing) and `public-sector` (government-sector
deployments). The public-sector profile REFERENCES the C7 trigger by
name — the first gov/bank pilot ignites the bit-for-bit toolchain
line (R38(8), the №913 report contract); the profile itself does NOT
ignite the trigger: declaring a vertical is a configuration act, the
trigger is an OWNER decision (the C7-связка: the trigger is not
ignited by the profile).

## §3. The parents are consumed by reference — what this record does NOT redefine

- The six contour modules and their machine facts — unchanged
  byte-for-byte: the profiles REFERENCE the contours (JurisdictionMatrix,
  APPROVAL_COVERAGE, REPLAY_MATCH, TAINT_SATURATION, LICENSED_FLOOR,
  №385), never restate, never re-derive, never widen a right
  (ADR-0203 stays the rights SSOT), never add a second egress gate
  (ADR-0207 stays the egress SSOT), never touch the frozen refusal
  vocabulary (№385 — the version-pin references it).
- The package-integrity gate (№911, ADR-0218) — unchanged; the
  vertical verdict consumes it BY ORDERING (integrity first,
  configuration second) and by the manifest SHAPE (the №910 [registry]
  form extended with `[vertical]`), not by re-implementing it.
- The digest-pin discipline (G5/№828/№868) — reused as-is; the profile
  carrier digest is the same SHA-256-over-canonical-serialization.
- The machine-fact discipline — gen_metrics + the gate files + the
  GENERATED rows + the tamper traps — reused as-is; the vertical fact
  is the next machine fact of the same family (the 14th).
- The institutional contour (ADR-0219) — consumed by reference: the
  vertical packages operate UNDER the legal shield; the documents ≠
  the owner's acts boundary (R38(9)) is inherited, not restated.

## §4. The consumption contracts — what №921–№922 land

1. **№921 — the kernel + the machine.** src/vertical.rs: VerticalId,
   VerticalContour, VerticalProfile (the fail-closed constructor — a
   profile without the full six contours does not construct), the
   append-only registry (payments, public-sector — the JurisdictionMatrix
   form №888), the carrier-digest pin (the canonical-serialization
   SHA-256), `vertical_verdict` (fail-closed consumption — unknown id /
   digest mismatch / unverifiable contour → the named refusal BEFORE
   effect; no default attach — the undeclared manifest verifies with
   zero behavioral mutations); the mlogpkg package path: the
   `[vertical]` manifest table (id + digest) + the verdict in the
   verify/build paths AFTER the №911 integrity pass; tests
   tests/naryad_921_vertical_profiles.rs: the negative controls (no
   declaration → 0 mutations; declared-but-unknown → refuse;
   broken digest → refuse; unknown id → refuse), the e2e (a package
   with `[vertical] payments` at the live pin → mlogpkg verify GREEN),
   the mutation probes (а)–(г) red-before-green. VERTICAL_PROFILES in
   gen_metrics.py + gate_045 (scripts/ci/gate_045_goals.txt — the
   explicit 12-element roster: id, carrier, anchor, sub-anchor; the
   blocking CI job; the blocking_checks.tsv row; the GENERATED rows in
   docs/metrics.md and docs/PLAN-SUMMARY.md; the double --check
   deterministic). The tamper traps (the next range after №46–№50):
   a profile removed from the registry → red; mandatory_contours
   emptied → red; a broken digest-pin → the consumption test red; a
   hand-edited GENERATED row → the double --check diverges. The
   ADR-definition trap: the removal of the `VERTICAL_PROFILES :=`
   literal from THIS record → red (the №916 mutation contract).
2. **№922 — the readiness report.** docs/audits/verticals-readiness-2026-11.md
   (the №907/№913/№919 form): the methodology, the 12/12 roster
   verdicts (every row = the CHECKED anchor pair of the live carrier),
   the repo↔owner-acts boundary with the owner's acts NAMED (the
   PSP/acquiring integrations, the government contacts, the
   certifications — PCI DSS and the regulator acceptances), the
   vertical distribution track as REGISTRY STRUCTURE (columns +
   readiness criteria, no stub records), the C7-trigger linkage (what
   ignites it, what does not — the №913 contract), the medicine
   backlog (R38(5)), the limitations/threat-model sync. The report is
   an ARTIFACT — the fact lives in the kernel (№921) and the machine
   (gate_045); deleting the report's boundary section does not red the
   machine, but the wave acceptance requires the report (the dispatch
   checklist).

## §5. Honest limits — what VERTICAL_PROFILES is NOT

- **Not certification.** The machine verifies the PRESENCE and
  STRUCTURE of the profiles and their integrity (the digest pins, the
  mandatory contours, the consumption refusals) — NOT their
  certification force. The machine fact says «the payments profile
  exists, carries the six contours, and refuses a mismatched
  declaration» — it does NOT say «this deployment is PCI DSS
  compliant» or «this deployment is accepted by any regulator». The
  certifications (PCI DSS, the payment licenses, the regulator
  acceptances) are the OWNER'S ACTS (R38(9)) — outside the
  repository, outside the naryads, outside this machine's claims.
- **Not legal advice.** The frame is the C10 verbatim (ADR-0209 §5):
  «конфигурация ограничений, не юридический совет». A vertical profile
  is a configuration of limits and mandatory contours; it does not
  counsel any external party on any legal obligation.
- **Not the owner's vertical acts.** The named acts that make a
  vertical REAL in the market — the PSP/acquiring integrations, the
  government contacts, the certification audits — live OUTSIDE the
  repository and are never opened as naryads; the repo carries the
  REGISTRY STRUCTURE of the distribution track (№922 §4), no records.
- **Not the run-path composition.** This wave lands the KERNEL and the
  PACKAGE-PATH consumption (mlogpkg verify/build); attaching the
  verdict to the interpreter's run loop and the server startup is the
  NEXT subphase's surface and is named here as the honest limit — the
  dispatch fixes the boundary, the code does not overclaim.
- **Not the medicine vertical.** Medicine is backlog until the PHI
  expertise exists (R38(5)) — an owner decision, not a roster item.
- **Not the C7 trigger.** The public-sector profile references the
  trigger; declaring the profile does not ignite the bit-for-bit
  toolchain line (R38(8), the №913 contract) — the trigger is the
  owner's decision on the first gov/bank pilot.

## §6. The migration discipline — 1:1, byte-identical surfaces

The wave touches one new src-module (src/vertical.rs — the registry
kernel, NOT the grammar, NOT the builtins), the mlogpkg manifest
parse + the two consumption sites, the metrics machine, and docs.
The grammar, the builtin registry (530 — does not grow), the golden
outputs, the frozen refusal vocabulary (№385), the six contour
modules' behavior — byte-identical (the surface gates verify
NO DRIFT; the №895/№908 ride form: the floors follow the fact, an
honest no-drift is a valid verdict). Cargo.lock grows by ZERO
entries (pure-std + the already-workspace-graph sha2, the
jurisdiction.rs/approval.rs form). The undeclared-path behavior
(the manifests without `[vertical]`) — ZERO behavioral mutations,
pinned by the negative control.

## §7. Acceptance

- ADR-0220 Proposed → Accepted at the wave-84 acceptance (the dispatch
  gh#1842 records the verdict).
- The gate 0.47: `gen_metrics --check` green with VERTICAL_PROFILES
  = 12 (the floor 12, only-up) on the wave merge chain; the tamper
  traps alive; the GENERATED rows synchronous; the double --check
  deterministic.
- The kernel in the tree with the two profiles × six contours; the
  fail-closed consumption green on the e2e and red on every named
  divergence class; the negative control (no declaration → zero
  behavioral mutations) pinned by test; the readiness report present
  (the wave acceptance's checklist); the grammar/builtins/golden and
  the six contour modules byte-identical (the git-diff proof on the
  merge chain).
