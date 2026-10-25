# ADR-0197: The VM Is the Only Executor of User Commands; TW Is the Reference Oracle (the C5 (b) decision, ACCEPTED 2026-10-23 by the owner's verbatim)

- Status: **Accepted** (2026-10-23 — the owner's verbatim in #1474,
  the decision line quoted in §2; the ADR-0193 §7 form — the decision
  as submitted is recorded, the machine adds no riders)
- Date: 2026-10-23 (the decision); 2026-10-24 (the ADR record, №791)
- Deciders: the OWNER (the C5 decision); the machine formats the record
- Naryad: №791 (issue #1514, Волна 63)
- Source: the audit-23.10 C5 (issue #1474) — the deliberation lives in
  the private strategy canon; this ADR records the decision and the
  machine-verifiable furniture only
- Supersedes/extends: the REMAINDER of
  [ADR-0105](0105-vm-experimental-scope.md) (already partially
  superseded by [ADR-0141](0141-vm-production-readiness.md)) — see §4;
  extends [ADR-0171](0171-serve-default-flip-vm.md) from `serve` to
  ALL the user-command lanes — see §4
- Scope: a DIRECTION record — it wires no CI and flips no default by
  itself; the landing rides the V-naryads (#1500, #1501, #1502, the
  waves 64–65 per the registry)

## §1. Context — the two-executor cost the audit measured

The audit-23.10 C5 finding: the tree-walking backend (TW) and the VM
both execute user programs today, and the dual execution is a
PERMANENT tax — every language feature lands twice (the lane-parity
form №762 exists precisely because the divergence class keeps
returning: gh#1336, gh#1430), every serve request carries the
`METALOGOS_SERVE_BACKEND` fork, and the diff-fuzzer (№465) exists to
catch what the two executors disagree on. The question the owner
answered: which executor becomes THE executor, and what the other one
becomes.

## §2. The decision (the owner's verbatim, #1474, 2026-10-23)

> **Решение владельца (23.10): вариант (b)** — *VM исполняет все
> пользовательские команды; TW — эталон-оракул в CI.*

The FULL verbatim lives in #1474 (the owner comment
2026-10-23T06:39Z — permanent, public); the ADR quotes the posture
line only (the docs-language budget — the lint is the honest machine,
the rest is linked, not paraphrased). The full verbatim additionally
fixes: the five command lanes named explicitly (`run`, `serve`,
`mcp-serve`, `eval`, REPL); the oracle's surfaces in CI AND behind the
`--reference` flag; the DEFAULT riders the owner accepted (correctable
by the owner) — `--reference` is TRANSITIONAL, two release windows;
the ORDER: after #1458 (№780 of Wave 62), V2 rides WITH the gateway
pilot (the G2 precondition #1481).

Interpretation (the record-keeping reading, the ADR-0195 §7 form): the
verbatim fixes FOUR things — (1) the VM is the ONLY executor of the
user commands across ALL the command lanes; (2) TW does not die — it
becomes the REFERENCE ORACLE: it runs in CI (the crosscheck, the
diff-fuzzer, the doc-tests) and behind the `--reference` flag; (3) the
`--reference` flag is TRANSITIONAL — two release windows by default,
an extension is a new owner act; (4) the ORDER is fixed: the
precondition #1458 landed (№780, `e77a93c`, Wave 62), and V2
(mcp-serve on the VM) rides WITH the gateway pilot (the G2
precondition #1481).

## §3. What stays (the oracle's contract)

The TW-as-oracle keeps its obligations — the oracle MUST understand
what it is the reference of:

1. **The core-semantics criterion stays:** "TW implemented | VM
   implemented" remains the acceptance form for the core-language
   constructions (the №462-era machinery; the oracle is not a
   strawman).
2. **The dedup continues:** the shared RuntimeContext (the decision
   4-A, the №540/№553 collapse line) keeps shrinking the twin bodies —
   the oracle and the executor share MORE code over time, not less.
3. **The oracle surfaces in CI stay:** the crosscheck battery, the
   diff-fuzzer (№465), the doc-tests — they compare the executor
   against the reference, they do not become user-facing lanes.

## §4. What this ADR replaces and extends (the ADR-lineage edits)

1. **ADR-0105 (the VM experimental scope) — the remainder is
   superseded.** ADR-0141 (the VM production readiness) already
   replaced the readiness posture; this decision removes the last
   standing piece — "the TW lane is the default executor" — by
   making the VM the only user-command executor. ADR-0105's file
   carries the superseded-remainder note (the same-PR edit).
2. **ADR-0171 (the serve default flip to the VM) extends from `serve`
   to ALL the user-command lanes.** The ADR-0171 contract (the VM is
   the serve default; the TW lane remains reachable) becomes the
   whole-command posture; the per-lane flips land with the V-naryads.
3. **The `--reference` flag** — the TW-oracle escape hatch for the
   users — is transitional (two release windows from the V3 landing,
   gh#1501); the extension is a new owner act (the same discipline as
   the ADR-0196 fork).

## §5. The landing order (the registry facts)

| Step | Issue | What lands | State |
|---|---|---|---|
| The precondition | #1458 / №780 | the runtime label gates execute in the pattern/route bodies on the VM | LANDED (`e77a93c`, Wave 62) |
| V-ADR — this ADR | #1499 / №791 | the decision record | LANDED (this naryad) |
| V2 | #1500 | `mcp-serve` executes the tool methods on the VM (the runtime labels in MCP) — WITH the gateway pilot (the G2 precondition #1481) | LANDED (№795, `a74d8dd`, Wave 64) |
| V3 | #1501 | `run`/`eval`/REPL on the VM + the transitional `--reference` (TW) | LANDED (№800, Wave 65 — the two-window transition clock starts at this landing) |
| V4 | #1502 | TW ONLY the oracle: the crosscheck/diff-fuzzer/doc-tests; the gate «the user path does not ride TW» | queued (Wave 66) |

The order is the owner's verbatim (§2): after #1458 (landed), V2 with
the pilot, then V3, then the V4 gate closes the lane.

## §6. The links

- The decision: #1474 (the C5 owner comment, 2026-10-23T06:39Z).
- The landing naryads: #1499 (this ADR), #1500, #1501, #1502.
- The lineage: ADR-0105 (superseded remainder), ADR-0141 (the
  readiness), ADR-0156 (the TW/VM JIT parity — the oracle's CI
  machinery), ADR-0171 (extended to all the lanes), ADR-0175 (the
  tick context), ADR-0194 (the loud-fail contract the VM lane keeps).
- The related decisions: ADR-0196 (the gateway pilot — V2 rides its
  G2 precondition), №787 of this wave (the [PRECISION_LOSS] boundary —
  the both-lane conversion contract it introduced is the oracle
  machinery working as designed).
