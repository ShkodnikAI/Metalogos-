# Risk-Review Checklist (№469)

Decision 7-A.2 (gate gh#680): the review goes **by risk, not by order**.
A PR that touches `src/io.rs`, `src/server.rs`, `src/audit.rs`,
`src/semantic.rs`, the `llm*` family, or **adds a `spawn`** must be
reviewed against this checklist. The mechanical surface (which changed
lines hit which item) is produced by `scripts/ci/risk_review.py --report`
and attached to the PR by the `risk-review` CI job.

**Status: ADVISORY (non-blocking).** The job never gates a merge by
itself; making it blocking is a separate owner decision. It does NOT
replace the completion audit (М3) — it is the per-PR risk surface.

**Not a self-check.** The agent that wrote the code does not sign the
checklist; the reviewer (a second agent run, the owner, or the future
second maintainer — №471) works through the items against the mechanical
report.

## The seven items

1. **Execution context.** Does the change touch a serve-route, cron or
   exec path? Every new/changed route and tick runs under its gate
   (`ServeRoute`, the cron/webhook guard of №457, the exec context of
   №455): confirm the gate is ON the path, not beside it, and the
   default context is the strict one.
2. **Result confidentiality labels.** Does the result of the new/changed
   path carry the right label (№322)? A `Secret` stays `Secret` (never
   materializes into a printable value); private data keeps its label
   through transforms; a canary path (№274) never strips markers.
3. **Default behavior — fail-open vs fail-closed.** Every
   `unwrap_or*`/default arm introduced by the diff: is the failure
   default CLOSED where the path is gated (env, read_file, egress,
   grants)? The September failures were silent fail-opens (audit 25.09
   §3.9); the strict-serve inversion of №457 is the baseline.
4. **NaN / empty / unbounded values.** Empty strings, empty lists, NaN
   and unbounded loops: does the path refuse loudly instead of
   producing a silent wrong value (the distillation NaN rule of №456 is
   the model)? Any new `unwrap()` on externally shaped input?
5. **TW/VM parity for stateful names.** If the diff mentions a builtin
   name literal: does the OTHER backend handle it the same way? The
   №462 counter holds the duplication fact; the №465 fuzzer holds the
   divergence classes — a new divergence is a separate owner-gated
   naryad, never a silent fix inside this PR.
6. **Core joints.** New routes, new `spawn`s, new filesystem/network
   surfaces, new effects: which lane do they cross (№316 Source/Sink)?
   Is the effect recorded in the Action Ledger where the class demands
   it (№393)? Does anything bypass an existing gate by going around it?
7. **The error surface.** Are the refusals LOUD (typed error, stderr
   warning, audit record) rather than a silent empty success? The
   fail-loud discipline of №454–№460 is the standard.

## The mechanical report

The CI job runs `scripts/ci/risk_review.py --report BASE HEAD` on every
triggered PR and publishes the table (item → file:line → observation)
as the job summary. The table is the MAP for the reviewer — the verdict
is the reviewer's, made here, item by item, with the file/line evidence.
The scan stays INSIDE the perimeter (the boundary rule of №469): the
perimeter files plus the `.rs` files that add a spawn.

## Escalation

An item the reviewer cannot close from the evidence becomes a note in
the PR and, when it is a real gap, a separate naryad (the boundary rule:
the review does not fix silently inside the reviewed PR).
