# Verticals-readiness report: the verified 12-element roster and the profile ≠ certification ≠ owner-acts boundary (the 8H-2 subphase)

**Naryad №922 (gh#1841, wave 84 — W-L2), class [docs][audit].**
The closing artifact of the vertical-profiles subphase (8H-2; C13а —
the verticals: payments + the public sector, VERTICAL_PROFILES — the
pre-last train slot before the 1.0 assembly). Every verdict below is
regenerable by the machine (`scripts/gen_metrics.py --check`, the
gate_045 roster) or readable verbatim in the named live source — no
prose verdicts.

**Report date:** 2026-11-06. **The tree:** main `d1066e6ae` (the
wave-84 merge chain). **The record:** ADR-0220 (the №920 subphase
record — the fact definition, the two verticals, the fail-closed
consumption, the honesty frame).

## 1. What VERTICAL_PROFILES counts — the machine verdict on this tree

```
OK: the №921 floor holds — VERTICAL_PROFILES 12/12 (gate_045 floor
12, only-up) — roster: 12 vertical anchors over the live kernel
```

The roster (`scripts/ci/gate_045_goals.txt`, the №862/№894/№918 form —
the count is CHECKED, not declared; a row counts only when its needle
(the profile-scoped contour anchor) AND its sub (the typed contour
variant) stand INSIDE the named seed region of the live kernel
src/vertical.rs, and the composition sites hold: the ADR-0220
definition literal (`VERTICAL_PROFILES :=`), the verdict function with
its digest-pin comparison, and the registry gathering both seeds):

| # | Item | Profile | The machine anchor (needle + sub, the seed region) |
|---|------|---------|------------------------------------------------------|
| 1 | the jurisdiction-coverage contour | payments | `payments contour (1) jurisdiction-coverage — the JurisdictionMatrix package, strictest-wins (ADR-0209)` + `VerticalContour::JurisdictionCoverage` (fn seed_payments) |
| 2 | the human-oversight/approval contour | payments | `payments contour (2) human-oversight-approval — the typed approval mandate (ADR-0205)` + `VerticalContour::HumanOversightApproval` (fn seed_payments) |
| 3 | the decision-trace verify contour | payments | `payments contour (3) decision-trace-verify — the replay/forensic ledger (ADR-0210)` + `VerticalContour::DecisionTraceVerify` (fn seed_payments) |
| 4 | the taint-floor contour | payments | `payments contour (4) taint-floor — the taint saturation lane (ADR-0217)` + `VerticalContour::TaintFloor` (fn seed_payments) |
| 5 | the license-class contour | payments | `payments contour (5) license-class — the license-typed model layer (ADR-0202)` + `VerticalContour::LicenseClass` (fn seed_payments) |
| 6 | the refusal-vocabulary version-pin | payments | `payments contour (6) refusal-vocabulary-pin — the frozen №385 dictionary, the version-pin` + `VerticalContour::RefusalVocabularyPin` (fn seed_payments) |
| 7 | the jurisdiction-coverage contour | public-sector | `public-sector contour (1) jurisdiction-coverage — the JurisdictionMatrix package, strictest-wins (ADR-0209)` + `VerticalContour::JurisdictionCoverage` (fn seed_public_sector) |
| 8 | the human-oversight/approval contour | public-sector | `public-sector contour (2) human-oversight-approval — the typed approval mandate (ADR-0205)` + `VerticalContour::HumanOversightApproval` (fn seed_public_sector) |
| 9 | the decision-trace verify contour | public-sector | `public-sector contour (3) decision-trace-verify — the replay/forensic ledger (ADR-0210)` + `VerticalContour::DecisionTraceVerify` (fn seed_public_sector) |
| 10 | the taint-floor contour | public-sector | `public-sector contour (4) taint-floor — the taint saturation lane (ADR-0217)` + `VerticalContour::TaintFloor` (fn seed_public_sector) |
| 11 | the license-class contour | public-sector | `public-sector contour (5) license-class — the license-typed model layer (ADR-0202)` + `VerticalContour::LicenseClass` (fn seed_public_sector) |
| 12 | the refusal-vocabulary version-pin | public-sector | `public-sector contour (6) refusal-vocabulary-pin — the frozen №385 dictionary, the version-pin` + `VerticalContour::RefusalVocabularyPin` (fn seed_public_sector) |

The tamper traps (`gen_metrics.py --tamper-test`, traps №51–№54) prove
each dispatch class red: the removed seeding, the emptied
mandatory_contours, the dead digest-pin consumption, the hand-edited
GENERATED row (and the dead ADR-0220 definition — the №920 mutation
contract). The floor is 12 only-up — the wave-84 dispatch constant
(gh#1842); a lowering is an owner transaction, never a gate edit.

The consumption is fail-closed and pinned by the real-binary e2e
(mlogpkg/tests/naryad_921_vertical_e2e.rs): a package declaring
`[vertical]` payments at the LIVE profile pin verifies GREEN
(keygen → pack → sign → verify); a broken digest refuses with
«vertical: vertical digest pin mismatch»; an unknown id refuses with
«vertical: unknown vertical id»; a manifest WITHOUT the `[vertical]`
table verifies with ZERO behavioral mutations (no default attach —
the negative control).

## 2. The two verticals as they stand — what the profiles are and are not

1. **payments.** A typed configuration of the six mandatory contours
   over the payment-processing deployment class (PSP/acquiring-grade).
   The profile REFERENCES the live parents by name — the
   JurisdictionMatrix (№888, strictest-wins), the typed approval
   mandate (ADR-0205), the decision-trace verify (ADR-0210), the
   taint floor (ADR-0217), the license-typed model layer (№841), the
   frozen №385 refusal vocabulary as the version-pin
   (`385-frozen-2026-11`); it carries the self-describing carrier
   digest pin (the G5/№828 form) and refuses any declaration pinned
   against a different profile generation.
2. **public-sector.** The same six-contour configuration over the
   government-sector deployment class. The C7-связка: the profile
   REFERENCES the repro-line trigger (the first gov/bank pilot —
   R38(8), the №913 contract) by name; the profile does NOT ignite
   the trigger — declaring a vertical is a configuration act, the
   bit-for-bit toolchain line starts only by the owner's decision.

Both profiles are CODE DATA in the append-only registry (the
JurisdictionMatrix discipline, №888) — added by code review, never by
configuration; the roster literals are verified against the canonical
derivation at construction (a drifted needle refuses).

## 3. The boundary: the repo carries configurations — the owner performs acts

The machine verifies the PRESENCE and STRUCTURE of the profiles and
their integrity (the digest pins, the mandatory contours, the
consumption refusals) — NOT their certification force (ADR-0220 §5).
A vertical profile is a configuration of limits and mandatory
contours — NOT a certification, NOT legal advice, NOT a regulator's
acceptance (the C10 verbatim: «конфигурация ограничений, не
юридический совет» — ADR-0209 §5). The following acts are named by
the plan (R38(9)) and live OUTSIDE the repository; none is opened as
a naryad, none has a stub record here:

| The owner's act | Status | What the repo carries |
|---|---|---|
| The PSP/acquiring integrations (the payment-rail onboarding, the merchant agreements) | the owner's act, outside the repo | the payments profile — the configuration of mandatory contours a payment deployment must satisfy |
| The government contacts (the public-sector pipeline, the procurement conversations) | the owner's act, outside the repo | the public-sector profile + the C7-trigger reference (R38(8)) |
| The certification audits (PCI DSS, the payment licenses, the regulator acceptances) | the owner's act, outside the repo | nothing — the machine fact says «the profile exists and refuses a mismatched declaration», never «certified» |
| The medicine vertical (until the PHI expertise exists) | backlog — the owner decision (R38(5)) | nothing (no profile, no roster row — medicine is not a variant in VerticalId) |
| The renaming №503 (Metalogos- → Metalogos) | the owner's act, strictly last | not started in this wave |

Reading any machine verdict as «the payments deployment is PCI DSS
compliant» or «the government pilot is accepted» would be a lie — the
gate counts CONFIGURATIONS.

## 4. The vertical distribution track — the registry structure (columns + criteria, no records)

The vertical partner track is the owner's time (R38(9)); the repo
fixes the STRUCTURE of the contact registry — the columns and the
readiness criteria — while the records themselves are owner acts. The
structure (the columns a future record fills):

- **The partner class** — the PSP/acquiring counterparties (the
  payments vertical), the government-sector contact points (the
  public-sector vertical), the certification bodies (both);
- **The target vertical / opportunity** — which profile the contact
  serves and what the deployment would consume (the six mandatory
  contours, the package path with the `[vertical]` pin);
- **The readiness criterion** — the machine-verifiable state that must
  hold BEFORE the contact: VERTICAL_PROFILES at the floor (12, only-up)
  on the live tree; the compliance package current (the №868
  re-attestation discipline); the INSTITUTIONAL_READY contour green
  (the legal shield precedes the first paying pilot — ADR-0219/№919);
  for the public-sector track — the C7 trigger decision recorded by
  the owner (R38(8));
- **The owner-act reference** — the contact itself, its date and its
  outcome live in the owner's records, never in the repo.

**No records exist yet** — the registry structure is empty by design;
a stub row here would be a lie about a contact that has not happened.

## 5. The cross-links (what this subphase consumes and does not ignite)

- **The C7 trigger (the №913 contract).** The bit-for-bit toolchain
  line and the Nix closure are built ONLY on the trigger — the first
  gov/bank pilot. The public-sector profile references the trigger;
  it does not fire it. The trigger is the owner's decision, recorded
  outside the repo.
- **The institutional contour (the wave-83 shield, ADR-0219/№919).**
  The legal shield (the trademark policy, the succession protocol,
  the neutral-structure frame) PRECEDES the first paying pilot; the
  vertical profiles consume it by reference — the documents ≠ the
  owner's acts boundary is inherited, not restated. The
  INSTITUTIONAL_READY fact holds its 12/12 floor on this tree.
- **The release chain (ADR-0218).** The Sigstore keyless posture («no
  human-held signing key») is the anchor the vertical packages'
  signing path already rides (v0.38.3 PUBLISHED 05.11); the
  package-path vertical verdict composes AFTER the №911 integrity
  pass — a tampered package refuses as INTEGRITY first.
- **The medicine backlog (R38(5)).** No PHI expertise exists in the
  office — the medicine vertical is not a variant, not a roster row,
  not a stub; it enters only by the owner's decision.

## 6. What the vertical coverage does NOT close (the honest limits)

- The gate reads the tree, not the world: a profile altered by a
  normal PR keeps its real-world meaning entirely outside the
  machine's claims — the tamper traps catch the SILENT structural
  damage (a removed seeding, an emptied contour set, a dead
  consumption site, a hand-edited GENERATED row), not the
  certification validity of any configuration.
- The run-path/serve-path composition (attaching the verdict to the
  interpreter's run loop and the server startup) is OUTSIDE the
  wave — the honest §5(4) limit of ADR-0220; the consumption today is
  the package path (mlogpkg build/verify).
- The mandatory contours verify the PRESENCE of the Phase-8 contour
  machinery a vertical deployment must satisfy — they do not EXECUTE
  the jurisdiction rules, the approvals, or the trace writes for a
  concrete deployment; those are the contours' own contracts (the
  parent ADRs).
- The external certification audit is not procured by this wave (the
  R38(3) owner gate with the reserved budget stands separately).

## 7. The docs sync landed in this naryad

- `docs/limitations.md` — the vertical-contour row: what the machine
  fact covers (the presence + the structure + the fail-closed
  consumption), what it does not (the certification force; the
  owner's acts; the run-path composition), the medicine backlog.
- `docs/threat-model.md` — the vertical row: what the fail-closed
  consumption catches (the unknown id, the digest mismatch, the
  unverifiable contour — each refusal named), what remains (the
  third-party certifications, the owner's acts, the run-path
  composition outside the wave).
