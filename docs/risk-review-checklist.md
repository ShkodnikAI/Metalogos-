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

## The nine items

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
8. **Same-effect paths (№480 — the audit 26.09 §4 method rule).** Before
   a naryad closes: find EVERY path that reaches the same effect the
   naryad fixes, and either close them at a COMMON POINT (the facade /
   the SSOT function) or LIST them in the report with the justification
   why they are not affected. A fix bound to a named call site closes
   an EXAMPLE, not the CLASS — the 26.09 High findings were both of
   that shape (`read_file` closed → `smtp_send`/pdf still open; the
   interpreter fixed → the VM kept the bug, №474). The working checks
   are seconds, not days — every completion report carries the THREE
   GREP PROTOCOL over the naryad's zone:
   - **files:** `grep -rn "std::fs::" <zone>` — every raw filesystem
     call either goes through the fs_gate facade (№475) or is listed
     with the justification;
   - **backend parity:** `grep -rn "<name>" src/vm.rs src/interpreter/`
     for every stateful name the naryad touches — the name lives on
     BOTH backends or the divergence is loud (the №462 counter);
   - **env without the prefix:** `grep -rn "ALLOWLIST\|<TAIL>" <zone>`
     — search env references by the substring WITHOUT the
     `METALOGOS_` prefix (e.g. grep `SERVE_ALLOW_ENV`, not
     `METALOGOS_SERVE_ALLOW_ENV`) — a partial/aliased reference
     (`env("DATABASE_URL")`) is invisible to the prefixed grep; the
     №758 class.

9. **The downgrade ledger (naryad №604 — the audit 25b375e §7 rule).**
   If the diff (or its naryad) LOWERS a blocking check to an advisory —
   a gate retired to a warning, an error re-classified, a severity
   dropped: the naryad must ENUMERATE the exact forms the check used to
   catch, and for EACH form show the evidence that the behavior is now
   correct (a test, a machine run, a parity proof). A form without its
   evidence STAYS BLOCKING — the downgrade covers only the enumerated,
   proven forms. The audit's lesson: №584 lowered the
   respond-terminality gate for ALL forms at once; the ONE form the
   lowering did not reach (a bare respond* nested under a top-level
   block-form if/else — the guard-bypass shape) silently lost its
   fail-closed refusal and became the Y-1 High regression, fixed by
   №600.


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
The mechanical report below also computes the three lists for the
diffed lines — the reviewer closes each entry or the report justifies
it.

