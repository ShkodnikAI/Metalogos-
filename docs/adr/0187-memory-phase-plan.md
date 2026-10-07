# ADR-0187: The Memory-Phase Plan — the Phase-4 slot, the container-form decision slot (ADR-0185 revisit), the parity posture

- Status: **Proposed** (the executor's draft plan; `owner_fixed: false` —
  the plan binds nobody until the OWNER fixes it; no implementation may
  start on this ADR alone)
- Date: 2026-10-07
- Deciders: the executor (the draft); the OWNER fixes the plan (the gate
  — open)
- Naryad: №624 (gh#1087, Wave 34)
- Predecessors: [ADR-0185 — the container form of memory_forget DEFERRED to the memory-phase planning](0185-memory-forget-container-deferred.md),
  ADR-0094 (the memory typology — phase 3 done), ADR-0135 (the semantic
  cache; the Phase-4 KNN foundation), ADR-0041/ADR-0066 (persistence,
  graph), №280 (the managed by-query forgetting surface)
- The wave rule honored: `memory_forget` is silently NOT fixed — the
  container form is worked out ONLY in this plan (ADR-0185 §3)

## §1. Context

ADR-0185 parked the container form of `memory_forget` as OPEN-BY-DECISION
and named the memory-phase planning its slot. The phase line: ADR-0041
(persistence) → ADR-0066 (graph) → ADR-0093/ADR-0094 (the typology —
**phase 3 done**) → ADR-0135 (**phase 4 scheduled** — the #272 KNN
infrastructure is its named foundation). The Known-Issue ledger gh#967 §1
(the lane-ranking / forget-recall-consistency / KG-recall findings) is
RESOLVED (№575, the four ignores lifted green) — the plan builds on the
closed lane, not on open findings. This ADR proposes the phase plan in
the draft-first shape (the №570/№615 precedent: the draft is the
executor's work; the fixation is the OWNER's gate).

**The draft binds nobody.** While `owner_fixed: false`, no realization
naryad may cite this ADR as its mandate; the realization naryads are born
from the plan AFTER the owner fixes it (ADR-0186 §5's owner-only path is
the shape).

## §2. The Phase-4 plan (DRAFT — the steps, not commitments)

| # | Step | The draft content | The depends-on |
|---|---|---|---|
| D1 | **The KNN recall reference** — the Phase-4 core | `recall` gains the KNN reference lane over the persisted memory graph where the corpus warrants it: the #272 embed/vec infrastructure (ADR-0135 D3 names it the Phase-4 foundation) ranks the candidate lanes BEFORE the lane-ranking heuristic; the full-scan cosine stays the reference implementation (the ADR-0135 boundary — revisited only if the tables grow); the recall output keeps the `[MEM]` provenance suffix | the owner fixes this plan; the vec feature |
| D2 | **The forget↔recall consistency across lanes** | the №575-closed lane contract (a forgotten node is not served by ANY lane) is stated as the phase invariant with the both-backend e2e pin (the state-accumulating memory rows in the serve-e2e inventory) | D1's lane work |
| D3 | **The container-form decision slot** (the ADR-0185 revisit) | the named container primitive vs the by-query-final question is DECIDED here — see §4; no container surface may land outside this slot | the owner's verdict |
| D4 | **The consent propagation design** (no code) | the mass-forgetting consent re-derivation at the container level: which consent classes survive a scope-wide forget, how the №280 bounds discipline (threshold / max_forget / dry_run / ids / FORGET_REASON) re-derives for a container, what the grant surface (the `memory_forget_cascade` grant — scope `memory:forget:<container>`) already covers | D3's decision |
| D5 | **The memory E2E inventory extension** | every state-accumulating memory declaration carries the both-backend run_test_server test (the §4.4 inventory posture, extended to the phase-4 rows) | D1/D2 |

## §3. The parity posture (the §13 rule, per step)

- **D1/D2/D5 — TW + VM**: the memory ops are TW+VM surfaces (the registry
  specs are backend-shared); the both-backend e2e is the acceptance, the
  diff fuzzer's memory group (№621: `memory_open/put/read/keys/release`)
  is the generative lane.
- **JIT — out**: the memory ops stay out of the JIT contour (the ADR-0073
  documented gap; the purity analysis excludes every non-arithmetic
  instruction — the №622 `SetValueReg` row keeps the mirror honest).
  The phase does NOT open the JIT memory question.
- **The VM cache boundary (ADR-0135 D3) stays**: `VM::call_llm` remains a
  separate surface; the semantic-cache loop is not migrated.

## §4. The container-form question (the decision slot's materials — the prices re-stated; the recommendation WITHOUT the owner's choice)

- **Option A — the named container primitive** (`forget-by-container`
  with cascade and consent semantics). Price: a new language surface
  (the registry row, the typed signature, the docs — the stage-0/1
  vocabulary has no container spelling yet, the №538 posture), the
  consent/security re-derivation (§2 D4 — mass forgetting touches the
  secrets/consent classes), the TW↔VM parity, the tests. And no consumer
  demand exists in the tree (the ADR-0185 recon: zero proposals, zero
  corpus usage — re-verified at the planning time).
- **Option Б — the by-query form as FINAL** (the №280 surface stays, the
  question closes). Price: near zero now; it bakes a "no" that Phase 4
  would re-litigate — exactly what ADR-0185 refused to do early.
- **Option В — the phased container**: D4's consent design lands FIRST,
  and the primitive decision re-runs on its facts (the container
  semantics ride the consent design, not precede it). Price: the
  decision moves one design-step later; the surface ships only with its
  consent story.

**The executor's recommendation (a draft, NOT a decision):** Option В —
the container semantics belong to the consent design's facts, and the
recommendation deliberately leaves A (never) and Б (now) open to the
OWNER. The by-query surface remains the accepted state until the
verdict (ADR-0185 §3).

## §5. The draft acceptance floors (X-3, owner_fixed: false)

The plan carries NO fact keys and NO gate record (it is an ADR, not a
gate — №525: a fact without a checker is forbidden; the phase's
acceptance floors are drafted strictly above the live facts and are
fixed by the OWNER together with the plan):

- D1/D2: the KNN-reference recall e2e on BOTH backends exists and is
  green (the live fact today: no KNN-reference recall lane exists — any
  floor demands movement);
- D5: the memory serve-e2e inventory — every state-accumulating memory
  row `done` on both backends (the live fact: the phase-4 rows do not
  exist yet);
- the ledger discipline: the phase opens no known-divergence pins
  (№503), the fuzzer's memory group stays live (the №621 liveness
  ratchet holds).

## §6. Honest limits

- **No implementations in this naryad** — the plan is prose; the code,
  the registry rows, the tests come from the realization naryads AFTER
  the owner fixes the plan.
- **No migration over phase 3** without the explicit steps above
  (ADR-0094's typology is the base, not a scratch).
- The draft's step order is the executor's arithmetic over the phase
  line, not a negotiated contour — the OWNER may fix any order, scope,
  or floors, including ones that demand MORE movement (the
  honest-direction bias; the ADR-0186 §6 shape).
- The ADR-0185 recon facts (zero demand) were re-verified at the draft
  time; they age — the decision slot re-runs the recon on ITS date.
