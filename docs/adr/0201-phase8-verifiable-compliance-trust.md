# ADR-0201: Phase 8 "Verifiable Compliance & Trust" — the Through-Line, the Subphase Train 0.35–0.47, and the ADR Booking 0201–0215

- Status: **Accepted** (the owner decision of 2026-10-28 — the plan
  revision R38 accepted in full, the owner's verbatim "I accept all of
  your recommendations"); the wave gate open: 0.34.0 PUBLISHED
  2026-10-28, verified via the releases/latest API by the R46 fact-check)
- Date: 2026-10-29
- Deciders: the OWNER (the R38 revision of the Phase 8 plan — the train,
  the audit budget, the legal shield, the verticals)
- Naryad: №836 (issue #1638, Wave 71 — W-A; the first naryad of the
  subphase 8D-1, issued under the owner directive to open the next
  wave of naryads per the strategic development plan)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `docs/projects/metalogos/language_strategy_plan_v2.md`, revision R38);
  the R46 GET-only fact-check over live main `d40aa28` (2026-10-29)
- Related: [ADR-0216](0216-rollback-metric-registry.md) (the booking note
  that fixed 0201–0215 as the Phase 8 reservation), [ADR-0200](0200-provable-lowering.md)
  (the provable tier-lowering line — the executed W-line this phase
  builds beside), [ADR-0196](0196-mcp-policy-gateway.md),
  [ADR-0197](0197-vm-single-executor.md), [ADR-0198](0198-error-model.md),
  [ADR-0199](0199-typed-share-by-tier.md) (the executed C-line parents
  of the audit-23.10 decisions), [ADR-0163](0163-backend-registry.md)
  (the license classes — the parent of the C4 line), [ADR-0155](0155-grant-algebra.md)
  (the grant algebra), [ADR-0167](0167-action-ledger-v1.md) (the Action
  Ledger v1), №495 (the distillation hub), №630 (the GREEN-gate pattern)

## §1. Context — why the phase needs one through-line ADR

Phase 8 "Verifiable Compliance & Trust" is the owner-accepted
development line that turns the language's existing security posture —
license classes, grants, the ledger, the audit gates — into a
**verifiable compliance story**: every claim the toolchain makes about
a model, a program, or an action is machine-checkable and carried by a
typed artifact rather than by prose. The strategic frame is the plan's
sentence: the language as a compiler of provable compliance wrapper —
the pillars (Entity/Pattern/Flow/Memory/Rule/Learn/Adapt) already give
the AI-native surface; Phase 8 gives the trust surface that makes the
result shippable to regulated consumers.

The phase is organized as **13 capabilities, C1–C13** (plan §23). Each
capability is a continuation of an already-executed subsystem — the
"outside the domain quota" property (the ADR-0177 freeze is lifted for
continuations, not for new islands):

| Line | Parent on main (verified) |
|---|---|
| C1 | The MCP policy gateway — [ADR-0196](0196-mcp-policy-gateway.md), №793–795, the G1–G5 gateway wave (№828 = the digest-pinning G5) |
| C2 | The grant algebra — [ADR-0155](0155-grant-algebra.md), Phase 3 |
| C3 | The typed-signature tier metrics — [ADR-0199](0199-typed-share-by-tier.md) |
| C4 | The distillation mechanics №495 + the license classes [ADR-0163](0163-backend-registry.md) — the License-Typed Model Layer (the 8D-1 subphase, this wave) |
| C5 | The VM single executor — [ADR-0197](0197-vm-single-executor.md) |
| C6 | The error model — [ADR-0198](0198-error-model.md) |
| C8 | The Action Ledger v1 — [ADR-0167](0167-action-ledger-v1.md), Wave 22 |

The remaining lines (C7, C9–C13) are enumerated in plan §23 with their
in-repo parents and facts; this public record references the plan
instead of copying the enumeration — the sanitization boundary (§5)
keeps the ADR to architecture, and the R46 fact-check verified the plan
carries the parent for every line. Re-stating unverified parents here
would be exactly the "README says X → proceed" posture AGENTS.md §1
forbids.

Until this record, **no ADR on main fixed the phase as a whole**: the
prior booking ended at 0200 (the provable-lowering line), and 0216 is
taken outside the reservation (the rollback-metric-registry record, its
booking note explains the gap). Without the through-line ADR, the code
naryads of the wave (№837–№842) have no process basis — the "ADR before
code" rule would have nothing to point at.

## §2. Decision — the through-line, the train, the booking

1. **The through-line is accepted.** Phase 8 proceeds as 13
   capabilities C1–C13, each landing as a typed, machine-checkable
   surface with its own ADR and its own machine fact — never as a prose
   promise.
2. **The subphase train 0.35–0.47 (the owner revision R38).** One minor
   release = one subphase + its machine fact:

   | Minor | Subphase |
   |---|---|
   | 0.35 | 8D-1 (C4 — the License-Typed Model Layer: ModelPassport, LICENSED_FLOOR, PASSPORT_FRESH) |
   | 0.36 | 8B-1 |
   | 0.37 | 8F-1 |
   | 0.38 | 8B-2 |
   | 0.39 | 8A |
   | 0.40 | 8C |
   | 0.41 | 8E-1 |
   | 0.42 | 8D-2 |
   | 0.43 | 8G |
   | 0.44 | 8F-2 |
   | 0.45 | 8E-2 |
   | 0.46 | 8H-1 (the legal shield) |
   | 0.47 | 8H-2 |

   The train order is the owner's decision (R38) and is not re-litigated
   per wave. The train ends at 1.0 (the plan's terminal stop).
3. **A subphase does not open until the previous one is GREEN** (the
   №630 pattern): the previous minor's machine facts are green on the
   live main before the next subphase's first naryad is issued.
4. **The "ADR before code" rule.** Every subphase's first naryad is a
   docs/ADR naryad that lands the subphase's ADR (from the booking
   below) before the first code naryad of that subphase. The rule is
   the plan's N1 rule, now pinned on main.
5. **The booking 0201–0215.** One ADR number per slot, assigned now:

   | ADR | Assignment |
   |---|---|
   | 0201 | This record — the Phase 8 through-line |
   | 0202 | The 8D-1 subphase record — ModelPassport, the passport as a type (issued by №837, the same wave) |
   | 0203–0214 | One per remaining subphase, in train order (0.36 → 0.47); the title of each is fixed by that subphase's issuing docs naryad, not pre-written here |
   | 0215 | The institutional contour — the phase's terminal record (the ADR-0216 booking note's "the institutional contour") |

   0216 stays the rollback-metric-registry record (outside the
   reservation, no collision — the ADR numbering gate holds).

## §3. The owner gates — the constraints this record fixes (no market content)

The R38 revision resolved four owner-level questions. This record fixes
them as CONSTRAINTS — the shape of each is public, the parameters stay
in the plan (the office canon):

1. **The train** — fixed (§2.2); re-ordering requires an explicit owner
   revision, not an executor's judgment call.
2. **The external audit budget** — a dedicated budget for the W-J
   external audit window is RESERVED by the owner. The public boundary:
   an external audit of the phase happens before the terminal stop; no
   figures, vendors, or scheduling live in this repository.
3. **The legal shield** — the 8H-1 subphase (the legal fine-reading of
   licenses, the compliance mapping) is scheduled at **0.46**, before
   the terminal stop; no license text or legal analysis enters the tree
   before its subphase (the ADR-0163 boundary stays: classes here,
   fine-reading there).
4. **The verticals** — vertical showcase launches happen by an explicit
   owner trigger per vertical, not by executor initiative; the phase
   lands the capability surfaces, the launches are the owner's act.

## §5. Sanitization boundary (the §16.0-7 rule)

The market sections of the plan (niches, economics, budget figures,
the R1–R13 forecasts) live in the office canon and are NOT published in
this repository. This ADR and every Phase 8 PR carry architecture,
train mechanics, and facts only. The sanitize-grep protocol runs in
every Phase 8 PR report (the wave acceptance: 0 hits in the diffs).
The public phase digest (PLAN-SUMMARY) is refreshed by a separate
wave docs naryad, not by the issuing naryad of an ADR.

## §6. What this record does NOT decide

- The ModelPassport details — the type, the validation, the hygiene:
  [ADR-0202](0202-model-passport.md) (№837) is the subphase record; this
  record only books its number.
- The floor targets of the 0.35 machine facts (LICENSED_FLOOR,
  PASSPORT_FRESH): the owner fixes targets at the subphase issuance
  (the wave dispatch's acceptance section); №841 lands the counters
  with start floors only.
- The vertical launches, the audit window schedule, the terminal-stop
  date: owner acts, per §3.
- Any change to the executed C-line parents (ADR-0196/0197/0198/0199/
  0163/0155/0167): Phase 8 builds ON them, never rewrites them.
