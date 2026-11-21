# E4 — the teacher `baseline_agreement` (№988, ADR-0200 §5)

- Status: measured where the machine honestly can; null WITH the reason
  where it cannot (the §7.3 encoding — nothing is invented)
- Date: 2026-11-21
- Naryad: №988 (issue #2022, dispatch wave В99 — W-AC, №AC1)
- Harness: `tests/research_e1_e4.rs` (env-guarded — the tests pass as loud no-ops in CI and measure only under METALOGOS_RESEARCH_E1E4=1; the documented
  invocation `scripts/research/run_e1_e4.sh`); the machine-readable
  companion is `e4-teacher-baseline-agreement.json`, the raw run log is
  `e1_e4_raw_run.txt`.

## The question

`ADR-0200 §5`, the E4 row: the share of the teacher's agreement with
the baseline on an eval set, per the teacher-registry records (№975),
with a separate row for the `training_on_output_allowed` flag. E4 is a
dependency of W6 (the differential enablement gate refuses
`unfiltered` models and needs to know what a teacher adds beyond the
majority baseline).

## The definition (recorded, so the number is reproducible)

agreement = the share of eval prompts where the teacher's answer string
equals the majority-baseline class word (and, separately, the TRUE
class word). The eval set: the same 400-text fixture (4 classes, seed
`20261121`); the prompt asks the teacher to answer with exactly one
class word; the baseline word is "alpha" (the balanced design ties
100/100/100/100; the FIRST max wins the tie — the rule is recorded in
the harness and in the E3 report).

## The results — one row per registry record

| backend | license class | output-training allowed | agreement (baseline) | agreement (truth) |
|---|---|---|---|---|
| mock | Osi | true | **0.0** | 0.0 |
| qwen25-vl-7b | Osi | true | null | null |
| claude-sonnet-4-20250514 | Restrictive | false | null | null |
| gpt-4o | Restrictive | false | null | null |
| llama3 | NonOsi | false | null | null |

- **mock — 0.0 BY CONSTRUCTION, deterministic (verified: every prompt
  answered twice, all answers identical).** The in-tree mock teacher's
  canned response is an FNV-1a hash token (`[mock-llm:<hex>]`) that
  never carries a class word — it cannot agree with any baseline or
  truth word. The mock is a plumbing fixture, NEVER a quality signal:
  the W6 enablement gate must not read its agreement as teacher value.
  That is the machine fact this row records.
- **qwen25-vl-7b — null with the reason:** a local-weights
  vision-language teacher; the weights artifact is not present in this
  sandbox (no HF download; the candle lane is off in the default
  build). The measurement requires the weights and the candle lane —
  run on a machine that holds the artifact to fill the row.
- **the three provider teachers — null with the reason:** closed or
  remote backends; the sandbox holds no network route and no provider
  credentials. The measurement requires a live call against the real
  backend — and decision 3 refuses distillation from them regardless
  (the row is provenance, not a permission).

## The honest reading

The sandbox can measure exactly ONE of the five registry teachers, and
that one is a placeholder whose agreement is zero by construction. E4's
machine fact for the line is therefore a boundary, not a number: **the
teacher-agreement input W6 needs cannot be produced in this sandbox —
it needs either the qwen weights on a holding machine (the only
permissive real teacher in the registry) or an owner-added permissive
provider record with credentials.** The null rows carry their reasons
verbatim in the JSON (the §7.3 form), the definitions and seeds are in
the harness — the measurement is one `run_e1_e4.sh` away wherever the
preconditions exist.
