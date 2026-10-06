# ADR-0184: The fate of the depth-≥2 early answer — `RESPOND_SWALLOWED` stays FOREVER (variant Б); the propagation-to-any-depth (variant А) is not built

**Status:** Accepted — the OWNER's decision, 2026-10-06. The authorization, verbatim: «1-Б, 2-3000, 3-отложить и добивай остальное» (the owner's chat; the decision marker `w31: depth2-decision` on the issue gh#1044, comment 6010561955; the executor's formulation, accepted as-is — the №598-pattern authorization, revocable by the owner).
**Date:** 2026-10-06
**Naryad:** №603 (issue #1044, Wave 31 — the OWNER GATE prepared by the naryad's recon; the decision taken in the owner's chat)
**Depends on:** №600 (gh#1041 — the blocking `RESPOND_SWALLOWED` refusal, PR #1054), №601 (gh#1042 — the rewired test contract), №584 (gh#998 — the VM respond-terminality lowering), ADR-0179 §4 (the release-block discipline), the audit 25b375e §3 Y-1
**Blocks:** nothing — the decision CLOSES the depth-≥2 semantics question; the deferred-revisit trigger is defined in §5

## 1. Context

The unified audit of `25b375e` (v0.28.1) found the Y-1 regression (High): №584
retired the `RESPOND_NOT_TERMINAL` gate for ALL bare-respond forms at once —
including the ONE form that never worked on EITHER backend: a bare
`respond*`/`respond_html*` call carried by a statement NESTED under a
top-level block-form `if/else` branch (depth ≥ 2). The audit's guard
scenario — `if is_admin(...) == false { respond("403") }` nested inside
another block-form `if` — executed the protected code for a NON-admin on
both backends (the serve lane's per-statement branch walk discarded
nested-statement responses). №600 restored the fail-closed posture: the
blocking `RESPOND_SWALLOWED` semantic error refuses every such program at
startup on BOTH backends; the SSOT `RespondPosition` predicate is shared by
the semantic walk and the №584 lowering; the migration is one word —
`return respond(...)`.

The naryad №603 posed the owner's choice between the two SAFE options the
audit named:

- **А. Propagate the early answer to ANY depth** — change the etalon: the
  interpreter throws `HttpResponse` out of the nested statements of an
  `IfElseBlock` branch as `Return`, the VM lowering repeats it. The guard
  code works without the explicit `return`.
- **Б. Keep `RESPOND_SWALLOWED` forever** — the form is simply FORBIDDEN;
  the early answer at depth ≥ 2 must be written `return respond(...)`.
  Zero new code; the compiler stays the reference enumerator.

The current posture (the advisory for the working forms + the blocking
error for the swallowed forms) is unsafe only toward "did not block" —
and that direction is already closed by №600 (fail-closed at startup).

## 2. The recon facture (the naryad's task 1 — the data the decision reads)

- **The corpus demand for variant А is ZERO.** №600's false-positive scan
  swept 363 `.mlog` files: zero new refusals — no real program carries the
  refused form. The executor's independent sweep of the examples corpus
  (2026-10-06): 8 example files carry indented `respond` lines (16
  occurrences), ALL of them are the working surfaces — bare `respond`
  directly in the route body (the №584 depth-1 surface, advisory-recommended
  `return`), `return respond(...)` forms, or the ONE deliberate pinned
  advisory fixture (`examples/compat/p69_deferred_response.mlog` — the №581
  sweep hit, kept unmigrated BY DESIGN with the explanatory comment: the
  deferred-response contract is the opposite of the return-form). The
  refusal's only carriers in the tree are the negative test fixtures.
- **The cost of variant А is a two-backend etalon change in the zone with
  three incidents.** Per the issue's own estimate: interpreter `eval_block`
  + the server per-statement loop + the VM lowering + the №465 fuzzer. The
  exact area produced X-1 (High, 0.28.1), Y-1 (High, 0.28.2), and the TW
  repair rider (the migration path itself silently broken on the TW since
  №584 — found by the №601-mandated test). The risk-adjusted price of А is
  therefore far above its ergonomic line item.
- **The migration cost of Б is one word**, already taught by the advisory
  and pinned end-to-end by №601 (403 to the non-admin BEFORE the protected
  code, on both backends).
- **The explicitness dividend.** The `return respond(...)` shape marks the
  route's exit point visibly — for guard/early-answer code this is a
  readability feature, not a tax; the language's security-first posture
  (№581, №600) treats the early answer as a CONTROL-FLOW TRANSFER, which
  the `return` keyword states honestly.

## 3. Decision

**Variant Б.** `RESPOND_SWALLOWED` stays FOREVER: a bare
`respond*`/`respond_html*` call carried by a statement nested under a
top-level block-form `if/else` branch (depth ≥ 2) is a STARTUP ERROR on
both backends, refuse-closed, with the stable `[RESPOND_SWALLOWED]` code.
The early answer at depth ≥ 2 must be written `return respond(...)`.
Variant А (the propagation to any depth) is NOT built.

The decision rule the audit prescribed holds as the standing criterion:
«понижая/запрещая — перечисли формы и докажи» (lowering or forbidding —
enumerate the forms and prove it). The enumeration for THIS decision:

- **The forbidden form (the blocking error):** a bare `respond*` /
  `respond_html*` call statement nested under a top-level block-form
  `if/else` branch — depth ≥ 2. Proof: the `RESPOND_SWALLOWED` startup
  refusal on both backends; the negative test pins the audit §3 guard
  scenario; the migration (`return respond(...)`) is pinned end-to-end.
- **The working forms (unchanged):** the bare `respond*` directly in the
  route body (the №584 depth-1 surface — the style advisory recommends the
  return shape); `return respond(...)` at ANY depth (the Return signal
  propagates — the repair-rider semantics); the top-level early answers
  the №584 lowering lowers.
- **The known surface not hosted:** the deferred-response contract ("code
  after respond() continues") — the opposite of the return-form, out of
  scope since №581, the pinned advisory fixture documents it
  (`compat/p69_deferred_response.mlog`).

## 4. Consequences

- No code change rides THIS ADR: the №600 posture is the decision's
  implementation; the tests, the advisory and the limitations row already
  on main ARE the accepted state.
- The depth-≥2 semantics question is CLOSED — it leaves the owner-gate
  queue and does not re-open by default (see §5 for the revisit trigger).
- The advisory (`return respond(...)`) remains the recommended shape for
  ALL early answers; the docs keep teaching the one-word migration.
- Variant А stays a documented non-option: any future proposal to propagate
  the early answer to any depth must come as a NEW ADR with the corpus
  demand proven (the zero-demand fact above is the baseline to beat) and
  the two-backend parity budget planned up front.

## 5. The revisit trigger

The question re-opens ONLY if ALL of the following hold: (1) real route
code (not fixtures) demonstrably trips `RESPOND_SWALLOWED` in the wild or
in the corpus growth — the demand the 2026-10-06 recon measured at zero;
(2) a 0.30-class planning slot explicitly budgets the etalon change
(interpreter + server loop + VM lowering + the №465 fuzzer); (3) the
depth-≥2 semantics owner-gate (№603's successor) is answered with the new
facts on the table. Until then the boundary stands.
