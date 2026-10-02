# ADR-0179: The 0.28 Release Gate — the Absolute-Goals Criteria (v2)

**Status:** PROPOSED — the executor's draft per №509 (the parameters are the owner's to fix — §3)

- Date: 2026-09-29
- driven by: №509 (gh#792); the audit 28.09 §4 + §3.4п3; supersedes the gate mechanics of ADR-0177 §6 for the 0.28 release (ADR-0177 stays the history and the §4 ratchets remain in force)

## 1. Context

The 0.27.0 release passed its gate 17–18 hours before the №482 CI gate
landed and before the duplicate counter reached zero. The root cause is
structural: the ADR-0177 §4 criteria measured the ABSENCE of regress
(ratchets: the type share "grows", duplicates "at/below the threshold",
debt "green") — none of them expressed a GOAL, and none of them looked
at open security findings or at whether the stateful features of `serve`
actually WORK end to end. A no-regression gate cannot fail a release
that merely does not improve; the audit's main lesson (the serving-path
distillation was dead behind green CI) is exactly this blind spot.

## 2. Decision

The 0.28 release gate (v2) = the ADR-0177 §4 ratchets (unchanged) PLUS
the ABSOLUTE goals, machine-checked by
`scripts/ci/unfreeze_gate.py --gate-target 0.28`:

1. **The typed-signature share REACHES the goal** (≥ `goal_typed_share_bp`
   basis points; draft 3000 = 30% — the audit 28.09 §4 recommendation;
   the live fact 9.96% at the ADR's writing). Compared against the GOAL,
   not against the no-regress threshold.
2. **Zero open High findings in the server path.** Every known High gets
   the `release-block` label on its issue; the count is synced into the
   fact record before the gate run (the label discipline — the checklist
   step); the gate fails while the fact > 0.
3. **The transfer domain quorum**: at most `goal_quorum_num/goal_quorum_den`
   (draft 1/3 — the external audit's recommendation, supported by the
   28.09 auditor) of the №466 groups still carry duplicate names.
4. **The №502 mirror metric** stays only-down (already folded into the
   §4.2 Dedup criterion — carried into v2 unchanged).
5. **The functional serve-e2e criterion**: every state-accumulating
   declaration (`distill_to`, `memory`, `conversation`, `cron`) has an
   end-to-end test through `run_test_server` on BOTH backends. The live
   inventory (`scripts/ci/serve_e2e_inventory.txt`): distill — done
   (№496); memory/conversation/cron — pending (follow-up naryads from
   the №509 report). Fail-closed while any row is pending.

Fail-closed everywhere: a missing record, a missing key, or an
unparsable value is a RED verdict, never a pass.

## 3. The owner's parameters (the owner gate — NOT the executor's)

The draft values live in `scripts/ci/gate_028_goals.txt`. The OWNER
fixes the final parameters in this ADR before the 0.28 window opens:
the share floor (30% draft), the quorum (1/3 draft), and confirms the
zero-High goal. The executor does not tune the goals; the executor
keeps the machinery honest and the facts synced.

## 4. The release-block label discipline

- Any known High finding in the server path gets the `release-block`
  label on its issue the moment it is triaged.
- Closing the finding closes the label.
- Before the gate run, the release manager syncs `fact_open_high_server`
  with the count of open `release-block` issues and records the sync in
  the release checklist (the checklist step below). The gate is
  machine-checked; the sync is a named human step with the evidence
  trail — the same posture as the office-e2e record of №482.

## 5. Mechanics

- Legacy mode (default): `unfreeze_gate.py --office-tests pass|fail` —
  the §4 summary, the 0.27.x read. Unchanged.
- v2 mode: `unfreeze_gate.py --office-tests pass --gate-target 0.28` —
  the §4 summary + the v2 goals section; the overall verdict is the
  0.28 release read.
- The summary artifact (`unfreeze-summary`) remains the one-page
  evidence; the v2 section lands in the same page.

## 6. The release checklist (the 0.28 window)

1. Sync `fact_open_high_server` with the open `release-block` issues
   (the count + the issue numbers into the gate record; the PR that
   syncs it is linked from the release notes).
2. Sync `fact_quorum_*` from the `count_duplicated_names.py` report.
3. Flip the serve-e2e inventory rows to `done` ONLY in the PRs that add
   the actual both-backend e2e tests.
4. Run the gate in v2 mode; attach the summary to the release.
5. The OWNER reads the verdict and decides the publication (the same
   owner-only posture as ADR-0177 §4 — the machine collects, the human
   decides).

## 7. Consequences

- The gate can now fail a release that is merely STABLE — by design.
  The 0.28 window opens with the v2 verdict RED (the share 9.96% < 30%,
  the quorum not reached, three serve-e2e rows pending) — that is the
  honest state, and the gate saying so is the feature.
- The ratchets (ADR-0177 §4) stay in force for the day-to-day merges;
  the goals add the direction. Neither alone is sufficient.
