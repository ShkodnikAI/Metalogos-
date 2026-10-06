# ADR-0185: The container form of `memory_forget` — DEFERRED to the memory-phase planning (not rejected); the managed by-query forgetting is the accepted surface

**Status:** Accepted — the OWNER's decision, 2026-10-06. The authorization, verbatim: «1-Б, 2-3000, 3-отложить и добивай остальное» (the owner's chat; the decision marker `w31: mforget-decision` on the issue gh#1050, comment 6010562323; the executor's formulation, accepted as-is — the №598-pattern authorization, revocable by the owner).
**Date:** 2026-10-06
**Naryad:** №609 (issue #1050, Wave 31 — the OWNER GATE prepared by the naryad's recon; the decision taken in the owner's chat)
**Depends on:** №280 (the `memory_forget` surface — managed forgetting with bounds), ADR-0041 (the memory persistence), ADR-0066 (the memory graph), ADR-0093/ADR-0094 (the memory typology, the recall — Memory Phase 3), ADR-0135 (the Phase-4 KNN reference), the audit 25b375e §6 item 10 (А+З)
**Blocks:** nothing — the decision CLOSES the audit's open question by recording the deferral

## 1. Context

The В31 unified audit's item 10 (the pair А+З) raised the "container form"
of `memory_forget` as an UNRESOLVED owner-level question, referencing
№503. The naryad №609's recon (the task 1) established the facture:

- **№503 (gh#786) itself does NOT carry the container question.** Its
  actual content is the fuzzer's outcome-divergence class (the
  ok↔err lanes moved to `BLOCKED_DOMAINS`, the two pinned divergences
  fixed) — closed with its own conditions met. The audit's reference is a
  lineage pointer, not a design proposal.
- **The "container form" exists ONLY in the audit's framing.** The repo's
  discussions, the limitations rows and the memory ADRs contain no
  container-forget proposal and no accepted or rejected decision — the
  question was never worked.
- **The live surface already hosts BOUNDED forgetting.**
  `memory_forget(db_path, table, query, threshold, max_forget[, dry_run[,
  ids]])` (№280) — managed forgetting by SEMANTIC MATCH with explicit
  bounds: the threshold, the max count, the dry-run preview, the explicit
  ids, the reason stamp (`FORGET_REASON`), the vec0-store-only scope. A
  "forget a whole scope" operation is expressible TODAY as a bounded
  query (a session/user/collection selector + a threshold) — what the
  container form would add is a NAMED PRIMITIVE for that shape, with its
  own semantics (cascade, atomicity, consent propagation).

## 2. The options and their prices (the naryad's task 2)

- **А. The container form as a builtin** — a named scope/container
  primitive (forget-by-container with cascade and consent semantics).
  Price: a new language surface (the registry row, the typed signature,
  the docs), the consent/security analysis (mass forgetting touches the
  secrets/consent classes — the №280 bounds discipline would need a
  container-grade re-derivation), the TW↔VM parity, the tests. And no
  consumer demand exists in the tree (the recon: zero proposals, zero
  corpus usage).
- **Б. Keep the current by-query form + a ratchet** — declare the
  managed-by-query forgetting the final shape. Price: near zero, but it
  bakes a "no" that the memory phase would then have to re-litigate —
  the memory phases (3 done, 4 planned per ADR-0094/ADR-0135) are exactly
  where container-grade semantics (scopes, ownership, consent
  propagation) naturally belongs.
- **В. Defer to the memory-phase planning** — record the question as
  OPEN-BY-DECISION, parked to the next memory-phase slot. Price: an ADR
  row; the question leaves the owner-gate queue without a premature
  "no".

## 3. Decision

**Variant В — DEFERRED.** The container form of `memory_forget` is
deferred to the memory-phase planning (the ADR-0094/ADR-0135 phase line —
the next slot is the Phase-4 planning). Deferred means NOT REJECTED: the
question is parked with this record as its anchor, and no container
surface may land outside that planning slot.

The accepted state until the revisit: `memory_forget` keeps its №280
managed-by-query surface (the bounds discipline: the threshold, the max
count, the dry-run, the explicit ids, the reason stamp, the vec0-only
scope); a "forget a scope" need is served by a bounded query, NOT by a
new primitive. No container builtin, no cascade semantics, no consent
re-derivation rides anything before the memory phase.

## 4. Consequences

- The audit's open question closes honestly: the decision EXISTS (the
  deferral), it is recorded, and it carries the trigger (§5) — not a
  silent non-decision.
- The owner-gate queue loses the item; the memory phase's planning (when
  it enters the wave queue) MUST re-open this ADR as one of its inputs.
- No limitations.md change: the current surface is the documented accepted
  state; nothing new is claimed.
- The corpus/tools are unaffected: no registry, grammar or CI change rides
  this ADR.

## 5. The revisit trigger

The next memory-phase planning slot (the Phase-4 line — ADR-0135's KNN
infrastructure or its successor). At that slot the container question is a
STANDING INPUT: the planner must either (1) scope the container form into
the phase with a consent/security re-derivation of the №280 bounds, or
(2) re-record the deferral with the phase's own facts. Outside that slot
the deferral stands unchanged.
