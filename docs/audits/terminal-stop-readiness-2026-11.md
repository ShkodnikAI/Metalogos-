# Terminal-stop readiness report: the 13/13 slot verdicts and the external-audit scope outline (the wave-85 W-T closing artifact)

**Naryad №925 (gh#1852, wave 85 — W-T), class [audit][process], strictly
last in the wave.**
The closing artifact of the terminal-stop wave (W-T — the start of the
1.0 assembly, the owner's directive of 2026-11-06): the verified
readiness of the 1.0 gate and the external-consumption evidence pack.
Every verdict below is regenerable by the machine
(`scripts/gen_metrics.py --check`, the gate_033–gate_046 rosters) or
readable verbatim in the named live source — no prose verdicts.

**Report date:** 2026-11-06. **The tree:** wave-85 W-T (ADR-0221 the
№923 record; TERMINAL_STOP the №924 machine — TERMINAL_STOP 13/13 on
the live tree). **The record:** ADR-0221 §2 (the gate definition, the
aggregate, the evidence shapes, the honesty frame).

## 1. Methodology — every verdict is machine-regenerable or a verbatim quote

- Every slot verdict regenerates by
  `python3 scripts/gen_metrics.py --check` (the exit 0 line «OK: the
  №924 floor holds — TERMINAL_STOP 13/13») and by the per-fact OK
  lines the same run prints (the gate_033–gate_045 floors).
- Every fact value quotes the GENERATED numbers block
  (docs/PLAN-SUMMARY.md, the `<!-- BEGIN GENERATED NUMBERS -->` block)
  — one generator, no hand-edited numbers (the №566/№900 posture).
- The tamper posture regenerates by
  `python3 scripts/gen_metrics.py --tamper-test` (the traps №55–№58 —
  the classes (а)–(г) of the wave-85 dispatch, plus the №841 family
  traps №1–№54).
- The prose claims (the owner-acts boundaries, the release path, the
  cross-links) quote the live records: ADR-0221, ADR-0201, ADR-0189,
  ADR-0219, ADR-0220, the plan §23 — each named, each readable
  verbatim in the tree.

## 2. The 13/13 slot verdicts (the ADR-0221 §2.1 map, every slot = the AND of its facts)

The machine verdict on the live tree:

```
OK: the №924 floor holds — TERMINAL_STOP 13/13 (gate_046 floor 13,
only-up) — the 13 capability slots of the Phase-8 train (ADR-0221 §2)
```

| Slot | Fact(s) (the AND) | The GENERATED verdict | The gate |
|---|---|---|---|
| C1 | ATTEST_MAP | 45 live attestation rows (floor 30) | gate_037 |
| C2 | GRANT_COVERAGE | 10000 bp (44/44) | gate_034 |
| C3 | PROVENANCE_SHARE | 10000 bp (6/6) | gate_038 |
| C4 | LICENSED_FLOOR ∧ PASSPORT_FRESH | 10000 bp (16/16) ∧ 10000 bp (16/16) | gate_033 |
| C5 | OFFLINE_SUITE | 10000 bp (12/12) (floor 8000 bp) | gate_039 |
| C6 | SEMA_EXTERNAL | 3 live external artifacts (floor 1) | gate_035 |
| C7 | REPRO_BITFORBIT | 10000 bp (11/11) | gate_043 |
| C8 | REPLAY_MATCH | 10000 bp (2/2) | gate_041 |
| C9 | TAINT_SATURATION | 324241748923742258252688 (floor 10⁹) | gate_042 |
| C10 | JURISDICTION_MATRIX | 3 (3/3) jurisdictions | gate_040 |
| C11 | APPROVAL_COVERAGE | 10000 bp (1/1) | gate_036 |
| C12 | REPRO_BITFORBIT (the shared fact) | 10000 bp (11/11) | gate_043 |
| C13 | INSTITUTIONAL_READY ∧ VERTICAL_PROFILES | 12/12 items ∧ 12/12 anchors | gate_044, gate_045 |

The aggregate counts a slot ONLY through the registered live fact
functions (the identity pin — a substituted checker never counts), the
fact's own roster strict-loaded (a dead roster is a refusal, never a
vacuous pass), and the ADR-0221 definition literal standing in the
record file. The traps №55–№58 prove the classes (а)–(г) red: the
fake-slot checker, the lost gate_046 roster, the dead ADR-0221
definition, the floor lowering. **What the machine says: the 13-slot
gate is GREEN on this tree. What it does NOT say: the audit is
procured, the pilot is paying, or the product is certified — §5.**

## 3. The external-audit scope outline — the funnel structure (no vendors, no figures, no schedule)

The outline names the audit SURFACES an external firm walks through —
per ADR-0201 §3.2, the vendor selection, the figures and the schedule
are the owner's acts and live outside the repository; the partner
track (R38(9) — the audit firms → C1) is the owner's time. The
structure below is what the repo carries so a future audit starts from
a defined scope, not a blank page:

1. **The surfaces (C1–C13):** the 14 machine facts of the table above
   with their rosters — the attestation map (C1), the capability
   disposition (C2), the boundary provenance (C3), the license class +
   the passport freshness (C4), the offline LLM surface (C5), the
   external-consumption artifacts (C6), the bit-for-bit registry +
   release digest chain (C7/C12 — the honest scope: the package path +
   the release digest, NOT the toolchain build — ADR-0218 §5), the
   decision-trace replay (C8), the taint saturation space (C9), the
   jurisdiction matrix (C10), the approval mandate (C11), the
   institutional contour (C13б), the vertical profiles (C13а).
2. **The ADR chain 0201–0221:** the through-line record (ADR-0201),
   the subphase records 0202–0211 and 0216–0221 — the design lineage
   an auditor reads first (each record: context → decision → honest
   limits).
3. **The machine facts gate_033–gate_046:** the roster files and their
   only-up floors — the reproduction instructions of every number an
   auditor verifies (`scripts/ci/gate_033_goals.txt` …
   `scripts/ci/gate_046_goals.txt`, the `--check`/`--tamper-test`
   modes).
4. **The readiness criteria (the pre-contact state, the №922 form):**
   TERMINAL_STOP at the floor (13, only-up); the compliance package
   current (the №868 re-attestation discipline); the institutional
   contour green (ADR-0219); the vertical profiles verified
   (ADR-0220).

**No vendor records exist here** — the funnel structure is empty by
design; a stub row would lie about a procurement that has not happened.

## 4. The evidence-bundle manifest — digest-pinned (the G5/№828 form)

The evidence bundle the owner hands to the auditor / pilot counterpart
carries THIS manifest; its digest is pinned at issue time and verified
by SHA-256 on re-verification (the content re-hash must match the pin;
a mismatched digest refuses — nothing else lives in the repo):

```
# evidence-bundle manifest — the №925 issue form (ADR-0221 §2.3)
scope:            the C1–C13 surfaces (14 machine facts, gate_033–gate_046)
adr_chain:        0201–0211, 0216–0221 (the Phase-8 through-line + the subphase records)
machine_verdict:  TERMINAL_STOP 13/13 (gate_046 floor 13, only-up) on the wave-85 tree
report:           docs/audits/terminal-stop-readiness-2026-11.md (this file)
audit_evidence:   the owner's bundle (the report contents live OUTSIDE the repo)
pilot_evidence:   the owner's record (the §5 form; the contract/invoice details OUTSIDE)
boundary:         the vendor/figures/schedule and the contract/invoice details are
                  the owner's acts — R38(3)/R38(9), ADR-0201 §3.2; the repo verifies
                  the digest pins, never the substantive force (§5)
manifest_digest:  sha256:53361c7e32fbb655628d8c9e7071b88ea95f27ebaba50d5011be0611a188d919
```

`manifest_digest` = sha256 over the manifest block above without the
`manifest_digest:` line (the pin binds the content, not itself):

**sha256:53361c7e32fbb655628d8c9e7071b88ea95f27ebaba50d5011be0611a188d919**

The re-verification: `sha256sum` the manifest block (minus the digest
line) and compare the hex string; a mismatch is a loud refusal — the
bundle content drifted after the pin.

## 5. The repo ↔ owner-acts boundary — what the wave does NOT procure

| The owner's act | Status | What the repo carries |
|---|---|---|
| The audit procurement (the vendor, the budget figures, the schedule) | the owner's act, outside the repo (R38(3) — the budget reserved; ADR-0201 §3.2 — no figures/vendors/scheduling) | the §3 scope outline + the §4 manifest form |
| The paying pilot (the counterparty, the contract, the invoice) | the owner's act, outside the repo (R38(9)) | the §5 pilot-evidence form (the digest pin only) |
| The pilot-evidence record | the owner writes it | **the form:** «the owner's record carrying the carrier digest» — the fields: the pilot date, the pilot product/scope, the counterparty class (never the name), the evidence-carrier digest (sha256 of the file the owner keeps), the record date. The record LIVES in the owner's hands; the repo carries the digest pin when the owner decides to publish it |

Reading the 13/13 verdict as «the audit passed» or «the pilot is
paying» would be a lie — the gate counts SLOT STATES, the machine
cannot manufacture the evidences (ADR-0221 §5.5).

## 6. The 1.0.0 release path — the calendar train stands

- The 1.0.0 minor departs the calendar train (ADR-0189 §5): the
  Saturday 12:00 MSK rc candidate from main (the unfreeze/lockstep
  gates + the rc tag + the draft with assets), the Sunday
  office-stand verification (the rc pin, the e2e battery, the `mlog
  check` baseline; a defect → the fix → `rc.N+1`), the Monday final
  tag ON THE SAME COMMIT — the release_train_gate verifies the final
  tag against the rc-candidate SHA.
- The publication is the OWNER's act (ADR-0186 §4.5); the empty-week
  rule stands (no evidence, no content, no departure — ADR-0189 §2.3).
- The precedent: v0.38.3 PUBLISHED 2026-11-05 (the №915 patch, the
  out-of-train publication — the №899 calendar-train amendment).
- The 1.0.0 notes list the Phase 8 subphases per §13.13.2 (the
  «Вошедшие подфазы» block) and the stand-verification result (the
  «Проверено на стенде офиса» line).

## 7. The cross-links (what this wave consumes and does not ignite)

- **The C7 trigger (the №913 contract).** The bit-for-bit toolchain
  line and the Nix closure are built ONLY on the trigger — the first
  gov/bank pilot (R38(8)). The 1.0 gate does NOT ignite the trigger;
  the trigger stays the owner's decision (ADR-0221 §5.4).
- **The institutional shield (the wave-83, ADR-0219/№919).** The
  legal shield PRECEDES the first paying pilot; the 1.0 gate consumes
  the INSTITUTIONAL_READY fact by reference (C13б) — the documents ≠
  the owner's acts boundary is inherited, not restated.
- **The verticals (the wave-84, ADR-0220/№922).** The profiles are
  configurations, never certifications; the 1.0 gate consumes
  VERTICAL_PROFILES by reference (C13а) — the profile ≠ certification
  ≠ owner-acts boundary is inherited, not restated.
- **The release chain (ADR-0218).** The Sigstore keyless posture is
  the anchor the 1.0.0 release rides; the release digest chain is the
  C7/C12 fact scope (the toolchain build stays behind the C7 trigger).

## 8. What the terminal-stop gate does NOT close (the honest limits)

- The gate reads the tree, not the world: the audit's substantive
  force and the pilot's business validity live entirely outside the
  machine's claims (ADR-0221 §5.1–§5.3); the tamper traps catch the
  SILENT structural damage (a dead anchor, a lost roster, a dead
  definition, a lowered floor, a fake slot), not the real-world
  validity of anything.
- The machine cannot manufacture the evidences: the 1.0.0 minor
  departs only after the owner holds both (ADR-0221 §5.5) — the wave
  schedules nothing.
- The 1.0 gate is NOT a certification, NOT a legal advice, NOT a
  regulator's acceptance, NOT a «quality as a service» guarantee (the
  C10 frame — ADR-0221 §5.2).
- The audit funnel (§3) is a structure without records: the vendor
  selection, the negotiation, the schedule — the owner's track
  (R38(9)), outside the naryads.

## 9. The docs sync landed in this naryad

- `docs/limitations.md` — the terminal-stop row: what the aggregate
  covers (the slot count, the identity-pinned checkers, the strict
  rosters, the digest pins), what it does not (the audit force, the
  pilot validity, the certification, the C7 trigger).
- `docs/threat-model.md` — the terminal-stop row: what the fail-closed
  aggregate catches (the fake slot, the lost roster, the dead
  definition, the floor lowering — each refusal named), what remains
  (the owner's acts, the external audit, the paying pilot).
