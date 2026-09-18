# Benchmark Run Protocol (Stage 4 re-run and all future benchmark naryads)

> **Status:** Accepted (naryad №398, issue #501). Mandatory for the Stage 4
> re-run and every subsequent benchmark naryad.
> **Donor:** alphaXiv/OpenResearch (`orx`, v0.2.4, MIT) — skill
> `agent-skills/orx-experiment-tree/SKILL.md`; study record `openresearch_tool_study.md` (Task 45).
> **Why:** naryad №381 (gh#467) ended with verdict **INSUFFICIENT DATA** —
> the runs normalized by a divisor (`per-request load_program`/clone chain)
> that was never declared, so the variants were not comparable. This
> protocol makes comparability mechanical.

## The five rules (verbatim contract)

1. **Fixed run contract.** One run command and an identical environment for
   all variants; between runs only COMMITTED code/config on the variant's
   branch changes. Env flags, mid-series command edits, and runtime hot
   patches are forbidden.
2. **Divisor policy.** The normalization divisor is declared BEFORE the
   run and printed in the final log line together with the raw numbers
   (raw AND normalized). Raw numbers are mandatory — a divisor may not
   absorb them (the direct answer to INSUFFICIENT DATA gh#467).
3. **Frozen variants.** The variant a run answered for (ANY result) is not
   edited afterwards; the next hypothesis is a child branch of it. A run
   that fails with an error that does not answer the node's hypothesis
   (OOM / timeout / infra) counts as a REPAIR of that node, not as its
   result.
4. **Repair cap / stop rule.** Two consecutive repairs without an answer
   on one node → pause and report to the owner. Three consecutive
   failures/regressions on a direction → stop the direction with a report.
5. **Stacked bushes.** A fan of variants is allowed only WITHIN one
   decision; the next round branches from the previous round's winner. A
   flat fan of all variants from the root, and a single chain of unrelated
   runs, are anti-patterns. The run-tree shape (parent → child list) is
   fixed in the report.

## Mechanical wrapper

`scripts/bench_run.sh` is the sanctioned runner for the Stage 4 benchmark
(`cargo bench --bench stage4_benchmark`):

- refuses to run unless the divisor is declared up front
  (`BENCH_DIVISOR=<positive integer>` — e.g. the corpus route count, 14);
- runs the ONE fixed command, unchanged, for every variant;
- extracts the raw numbers from the bench's JSON report and prints the
  mandatory final line: `RAW … | DIVISOR … | NORMALIZED …`;
- appends one `parent → child` line per run to
  `docs/research/bench-tree.log` (the run-tree form the report requires).

The first executed series under this protocol (naryad №398 criterion (б)
and (в)) is recorded in
[`docs/research/bench-protocol-first-series.md`](bench-protocol-first-series.md).

## Report form (every benchmark naryad's report must contain)

```
decision: <one sentence>
tree: root -> <variant A> ; root -> <variant B> ; <winner> -> <child …>
divisor (declared before the runs): <what>/<value>
per-node verdict: <variant> -> promote | repair(n) | stop, with raw+normalized numbers
```
