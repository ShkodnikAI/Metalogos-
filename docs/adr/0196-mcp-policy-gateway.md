# ADR-0196: The MCP Policy Gateway — the flagship pilot (the C1 (b) decision, ACCEPTED 2026-10-23 by the owner's verbatim)

- Status: **Accepted** (2026-10-23 — the owner's verbatim in #1470,
  the posture line quoted in §2; the ADR-0193 §7 form —
  the decision as submitted is recorded, the machine adds no riders)
- Date: 2026-10-23 (the decision); 2026-10-24 (the ADR record, №790)
- Deciders: the OWNER (the C1 decision); the machine formats the record
- Naryad: №790 (issue #1513, Волна 63)
- Source: the audit-23.10 C1 (issue #1470) — the deliberation lives in
  the private strategy canon (the plan-branch rule: the market
  analysis, the option weighing and the priority reasoning are NOT
  restated in public issues; this ADR records the decision and its
  machine-verifiable furniture only)
- Scope of this ADR: a DIRECTION record. It wires no CI, changes no
  runtime, and opens no code surface by itself — the pilot steps
  G0–G4 carry their own naryads (#1458, #1460 landed in Wave 62;
  #1480–#1483 queued in the wave registry, waves 64–66)

## §1. Context — the product-direction question the audit posed

The external audit of 2026-10-23 (revision `7c2276c`) posed the C1
question: which surface of the Metalogos estate becomes the FIRST
product — the language itself, or the MCP policy gateway built on top
of the label/ledger/label-runtime machinery the language already
runs. The audit's split listed three postures: (a) the gateway is the
product and the language is its policy DSL, (b) the gateway is the
FLAGSHIP ON TRIAL while the language remains a product, and (c) no
product posture yet. The deliberation (the private canon) priced the
options against the wave-62 security landing (the A-epic: the runtime
label gates in the pattern/route bodies — №780; the escape_html label
discipline; the resource limits), because the gateway's promise —
"an agent cannot leak, and the refusal is recorded" — is only honest
after that landing.

## §2. The decision (the owner's verbatim, #1470, 2026-10-23)

> **Решение владельца (23.10): вариант (b) — шлюз как флагман на
> пробу.** *Dogfood подтверждён владельцем (23.10): Fosved Office —
> первый пользователь шлюза.*

The FULL verbatim lives in #1470 (the owner comment
2026-10-23T05:39Z — permanent, public); the ADR quotes the posture
line and the dogfood confirmation only (the docs-language budget —
the lint is the honest machine, the rest is linked, not paraphrased).

Interpretation (the record-keeping reading, the ADR-0195 §7 form): the
verbatim fixes THREE things — (1) the posture is (b): the gateway is a
PILOT ("on trial"), not a pivot; (2) the language remains a product
(the verbatim says so explicitly) until the fork — the pilot does not
demote it by itself; (3) the exit is MEASURED by the §4 criteria
(the demo published; the Fosved Office dogfood; ≥ 3 external
installation attempts) — the fork after the pilot is a NEW owner act
either way (the expansion (a) or the course unchanged (c)). Nothing in
this ADR changes the wave registry, the gates, or the language roadmap
by itself.

## §3. The pilot scope — G0 → G4

| Step | Issue | What it delivers | State |
|---|---|---|---|
| G0 — the precondition | #1458, #1460 | the runtime label gates execute in the pattern/route bodies; the resource limits | LANDED (№780, `e77a93c`; #1460 in Wave 62 queue) |
| G-ADR — this ADR | #1479 / №790 | the decision record with the measurable exit | LANDED (this naryad) |
| G1 | #1480 | the MCP client: the long-lived upstream sessions, the current protocol | queued (Wave 64) |
| G2 | #1481 | the floating session label — the data flow between the tool calls | queued (Wave 65) |
| G3 | #1482 | the ledger entry on EVERY tools/call (allowed / refused / reason) | queued (Wave 64) |
| G4 | #1483 | the demo + the dogfood: an agent tries to leak a secret, gets the refusal AND the ledger record | queued (Wave 66) |

The G0 precondition is honest: the wave-62 A-epic (the dispatch
gh#1490) landed the runtime label gates on the serve path — the №780
merge commit `e77a93c` carries the both-loop execution; #1460 (the
resource limits) rides the same wave queue. The G-steps keep the
wave-form rules (the red test first where the step touches the
runtime; the loud contract ADR-0194 (c); the floors only-up).

## §4. The exit criteria (measurable — the pilot's contract)

The pilot SUCCEEDS (and the fork opens) when ALL THREE hold:

1. **The G4 demo is published** — the video AND the reproducible
   script (the script runs on a fresh clone; the video shows the
   refusal and the ledger record live).
2. **Fosved Office works through the gateway** (the dogfood) — the
   office's own agents pass their tool calls through the Metalogos
   gateway in daily operation, not in a staged demo.
3. **≥ 3 external installation attempts** — issues, discussions, or
   PRs from people OUTSIDE the project who tried to install/run the
   gateway (the count is the public tracker's, not the maintainer's
   inbox).

The criteria are conjunctive on purpose: a demo without the dogfood is
a staged act; a dogfood without the external attempts is a private
convenience; the external attempts without the demo cannot exist.

## §5. The fork by the results — a NEW owner act

When the §4 criteria resolve, the owner picks (the decision is NOT
pre-delegated):

- **(a) the expansion** — the gateway becomes the product; the
  language repositions as its policy DSL (the C1 option (a));
- **(c) the course unchanged** — the pilot closes, the gateway lands
  as a capability, the language keeps the product posture (the C1
  option (c)).

Both outcomes are honest exits. The pilot's failure mode is NOT
"the criteria unmet by the deadline" — it is the criteria unmet AND
unmeasurable (the demo unrunnable, the dogfood unprovable); that state
routes back to the owner with the facts, not to a silent extension.

## §6. The boundaries (what the pilot does NOT do)

1. **No builtin-surface expansion beyond the gateway's need** — the
   G-steps add no language builtins the gateway does not consume; the
   language enrichment protocol (ADR-0110) keeps its gate.
2. **The domain quota and the generative stop-list stay in force**
   (the wave-form rule; the §16.0-D no-stub rule applies to the
   G-steps as to everything).
3. **The language stays a product until the fork** — no G-step may
   deprioritize the language roadmap items (the goal032 lanes, the
   release trains) to serve the pilot; the waves keep their own
   compositions.
4. **The public issue hygiene (the owner rule 23.10):** the pilot
   issues carry the task, the code anchors and the acceptance criteria;
   the market reasoning stays in the private canon.

## §7. The links

- The decision: #1470 (the C1 owner comment, 2026-10-23T05:39Z).
- The pilot naryads: #1479 (this ADR), #1480–#1483 (G1–G4);
  the G0 precondition: #1458 (landed, №780), #1460.
- The related decision set: #1471 (C2, the positioning — the README
  first-screen lands as #1485), #1472 (C3, the maturity tiers —
  #1487), #1473 (C4, the process cost — №788/№789 of this wave),
  #1474 (C5, the single reference backend — the ADR-0197 class),
  #1475 (C6, Int/Result-Option — №787 of this wave + the ADR-0198
  class).
- The wave registry: FOSVED-office-v2
  `docs/projects/metalogos/wave_registry_2026-10.md` (the C4 form —
  the waves 63–66 rows).
