# ADR-0183: The migration-rollback boundary — schema evolution stays ADDITIVE-ONLY; the rollback path is backup restore, not a language surface

**Status:** Accepted — the DECISION DELEGATED by the OWNER, 2026-10-05. The owner's operative authorization, verbatim: «делай как ты считаешь правильным и начинай волну 30» — the FULL sentence is preserved in the GitHub records (the PR #1037 description and the naryad issue #1018 report; quoted there in full). (the №598-pattern authorization: revocable by the owner, who retains the signature right; the variant chosen by the executor per §4; the Volna-30 gate acceptance rides on this record)
**Date:** 2026-10-05
**Naryad:** №593 (issue #1018, Volna 30 — the OWNER GATE: the choice A/B, before the Kamerton wave 1 starts)
**Depends on:** ADR-0060 (the schema-as-code contour: DDL-replay ADDITIVE-ONLY), ADR-0175 §3.4 (drop/alter declared OUTSIDE schema-as-code), ADR-0155 (irreversible SQL requires Value::Grant), №390/№412 (the grant scope/TTL machinery)
**Blocks:** the Kamerton wave 1 start (the Н1-03 gate) — RESOLVED 2026-10-05 by this ADR

## 1. Context

The first external consumer of the language (the Kamerton project) carries an
acceptance criterion Н1-03: a migration applies AND rolls back idempotently.
The language contour cannot satisfy this today, BY DESIGN:

- ADR-0060 fixed the schema-as-code contour as DDL-replay, ADDITIVE-ONLY
  (`CREATE TABLE IF NOT EXISTS`);
- ADR-0175 §3.4 declared DROP and ALTER outside schema-as-code — the
  destructive-DDL class is deliberately not hosted;
- ADR-0155 and the №390/№412 line: an irreversible SQL statement requires a
  `Value::Grant` with scope coverage (a runtime property) and Once-linearity
  (a static property).

The naryad №593 posed the owner's choice:

- **A.** `migrate_down` under a grant: scope required, `GRANT_SCOPE_MISMATCH`
  and TTL covered by mutation checks — the destructive path AUTHORIZED, the
  engineering surface = the DB builtins module + tests + this ADR;
- **B.** Rollback not supported: a `docs/limitations.md` row and the
  Kamerton Н1-03 acceptance criterion amended to "applies idempotently, the
  rollback goes through backup restore" — the boundary DECLARED, the surface
  = documents only.

## 2. Decision

**Variant B.** The migration-rollback path is NOT built into the language.
The schema-as-code contour stays ADDITIVE-ONLY, and the consumer's rollback
requirement is satisfied by the standard recovery discipline: idempotent
up-migrations in the language, rollback through backup restore outside it.

The Kamerton Н1-03 acceptance criterion is amended to its canonical form:

> **Н1-03 (amended, ADR-0183):** "the migration applies idempotently (a
> repeated replay run changes neither the schema nor the data); the rollback
> goes through backup restore owned by the operating environment, outside the
> language contour."

The canonical text of the amendment is recorded on the dispatch thread
gh#1022 (in the consumer-facing language) — this ADR is the canonical
decision record; the two texts agree.

## 3. Why not A (the considered alternative)

The honest weighing, not a formality:

1. **`migrate_down` is the DROP/ALTER class by another name.** A down-migration
   is schema destruction with extra steps: it removes columns, tables,
   constraints. ADR-0060/ADR-0175 §3.4 excluded exactly this class — not
   because it is hard to authorize, but because hosting it makes the language
   the instrument of its own data loss. Re-admitting it under a grant flips a
   load-bearing architecture invariant for one consumer criterion.
2. **A grant solves AUTHORIZATION, not blast radius.** The №390/№412 machinery
   answers "who has the right"; it says nothing about "what does this
   down-migration do to the live data". A wrong or stale down-migration under
   a perfectly valid grant destroys the same data as without the grant. The
   mutation-test surface (scope, TTL) would pin the AUTHORIZATION gates —
   green tests, unchanged blast radius.
3. **The consumer's real requirement is recovery, not rollback-in-language.**
   Idempotent up-migrations + a verified backup/restore discipline is the
   standard practice for schema evolution at this scale; it keeps the
   destructive step where review and tooling actually live (the operator
   side). Nothing in Н1-03's use cases needs a programmatic `migrate_down`.
4. **The containment rule of the wave.** The dispatch's standing "do not do"
   rule keeps the contour from expanding beyond the listed surface ("no new
   media paths… the domain lines do not open inside a wave"); building a new
   destructive capability to satisfy one acceptance clause is the opposite
   direction.
5. **The honest-boundary protocol (№588) exists for exactly this shape:**
   declare the boundary loudly in `docs/limitations.md`, amend the criterion,
   keep the surface small — versus building capability and carrying its
   security surface forever.

## 4. The delegation record

The Volna-30 dispatch (gh#1022) listed №593 as the owner gate. The owner
delegated the decision on 2026-10-05 — the verbatim sentence is preserved in
the Status line above (the only Cyrillic quotation in this record; the same
verbatim text lives in the GitHub records: the PR #1037 description and the
naryad issue #1018 report). The executor chose B per §3. This record follows
the №598 precedent: the authorization quote is fixed verbatim, the executor's
drafting is named, the owner retains the signature right — the ADR stands as
Accepted-by-delegation and is revocable by a later owner decision (which
would re-open №593 as variant A with its own engineering naryad).

## 5. Consequences

**Positive:**

- the ADDITIVE-ONLY invariant (ADR-0060, ADR-0175 §3.4) holds without
  exceptions — the destructive-DDL class stays outside the language surface;
- no new grant scope, no new mutation-test surface, no new DB builtins — the
  security perimeter does not grow;
- the Kamerton wave 1 is unblocked (the Н1-03 gate RESOLVED) with the
  criterion the consumer can verify idempotently and recover from.

**Negative (the honest boundary, carried in `docs/limitations.md`):**

- a program cannot roll its own schema back; the recovery path is the backup
  restore discipline owned by the operator environment;
- if a future consumer demonstrates a NEED for programmatic down-migrations,
  the variant A path (grant-gated `migrate_down` with the scope/TTL mutation
  surface) remains the designed answer — as a NEW naryad re-opening this ADR,
  not an exception.

## 6. Implementation (variant B, this naryad)

- this ADR (docs/adr/0183-migration-rollback-boundary.md) — the decision and
  the amended criterion;
- `docs/limitations.md` — the new row in the same PR (the №588 protocol): the
  rollback boundary, the primary sources, the re-open condition;
- the dispatch thread gh#1022 — the amendment comment on the Kamerton wave 1
  gate;
- the naryad issue #1018 — the gate-resolution report.

No code, no registry, no tests are touched — variant B's boundary is
documents only (the naryad's own scope row).
