# The taint saturation audit: the wave-81 method, the evidence, the honest limits

**Naryad №907 (gh#1801, wave 81 — W-J, P2, class [security][docs]).** The
audit-report artifact of the saturation subphase (8F-2; the release-train
binding was replaced — the ADR-0189 amendment, №899: «TAINT_SATURATION
≥ 10⁹ + the audit report», the dispatch [gh#1803](https://github.com/ShkodnikAI/Metalogos-/issues/1803)).
This is the INTERNAL saturation audit — the method, the evidence and the
honest limits of the wave-81 taint lane — NOT an external paid audit (an
external audit is the OWNER's gate, the R38(3) budget line, outside this
pipeline; see §6).

**Report date:** 2026-11-04. **The audited main:** `b0e73e6a` (the №904
merge) + the №905 ratchet (PR #1806) + the №906 machine (PR #1807) — the
wave-81 chain in dispatch order. Every number below is REPRODUCIBLE from
the commands in §5 — none is declared by hand.

## 1. The method — the ADR-0217 §2 formula, 1:1, over live anchors

TAINT_SATURATION is defined (ADR-0217 §2, the №903 record) as the EXACT
count of distinct taint-relevant programs the wave-81 generator can emit —
an integer product over LIVE anchors, never declared:

```
Space = ScenarioVariants × SequenceSpace
ScenarioVariants = T × L × S × K × Z
SequenceSpace = Σ_{k=0..B} F^k
```

| Factor | Meaning | Live anchor (the machine reads the tree, not a claim) |
|---|---|---|
| T = 8 | the scenario templates of the №904 roster | the quoted region between the `№904 taint roster BEGIN/END` markers in `tests/naryad_465_diff_fuzzer.rs` (the №862 quoted-region form) |
| L = 3 | the live label lanes | the PRESENCE of `src/html_label.rs`, `src/sql_label.rs`, `src/secret_label.rs` — a removed lane module ZEROES the factor |
| S = 6 | the live sources | the roster's distinct source fields (`env`, `string_literal`, `concat`, `ident`, `literal`, `call_llm`), each cross-checked against the live label/classification/audit files — a dead name zeroes the factor |
| K = 11 | the live sinks | `SINK_FUNCTIONS` (`src/audit.rs` — 6 names, the №565 list) + `SQL_ARG_POSITIONS` (`src/builtins_classification.rs` — 5 sql entry points, the №831 SSOT) |
| Z = 1 | the live sanitizer | the `escape_html` classification entry (`src/builtins_classification.rs:142`, the Sanitized stamp) — a removed entry zeroes the factor |
| F = 6 | the statement forms | the `FORM_*` roster of the live fuzzer source (`FORM_FLAT..FORM_MATCH`) |
| B = 26 | the statement budget | the `let mut budget = 26u32` constant of the same file |

The arithmetic is exact integer arithmetic (python int — no overflow):

```
ScenarioVariants = 8 × 3 × 6 × 11 × 1 = 1584
SequenceSpace    = (6^27 − 1) / 5     = 204,698,073,815,493,849,907
Space            = 1584 × 204,698,073,815,493,849,907
                 = 324,241,748,923,742,258,252,688   (≈ 3.24 × 10²³)
```

The floor is the wave-81 dispatch constant: **10⁹ only-up**
(`scripts/ci/gate_042_goals.txt`, `goal_taint_saturation_min 1000000000`;
a lowered floor is refused loudly — the №776 discipline). The live product
exceeds the floor by a factor of ≈ 3.2 × 10¹⁴.

## 2. The evidence — the run (the №904/№905 lane) and the machine (№906)

### 2.1 The per-run triple roster (the №904 lane, the №905 ratchet)

Every roster scenario runs through BOTH backends (TW + VM) on every CI
run; the outcomes must agree and match the expected refusal CLASS:

| # | id | triple (lane/source → sink) | expected |
|---|---|---|---|
| 1 | `sec_print_env` | secret/env → print | refuse:SECRET_LEAK |
| 2 | `sec_writefile_env` | secret/env → write_file | refuse:SECRET_LEAK |
| 3 | `sec_print_control` | secret/string_literal → print | serve |
| 4 | `sql_query_scalar_derived` | sql/concat → query_scalar | refuse:SQL_DYNAMIC |
| 5 | `sql_dbinsert_dynamic` | sql/ident → db_insert | refuse:SQL_DYNAMIC |
| 6 | `sql_query_literal` | sql/literal → query | serve |
| 7 | `html_respond_llm_raw` | html/call_llm → respond | refuse:HTML_INJECTION |
| 8 | `html_respond_escaped` | html/call_llm → respond | serve |

8 roster rows → **7 distinct (lane, source, sink) triples** (rows 7–8 share
`html/call_llm → respond` — the refusal and its clean twin). The №905
ratchet pins the triple count ONLY-UP
(`scripts/ci/fuzz_taint_triples_baseline.txt` = 7, the №766 form) with the
LIVENESS guard (an empty roster is RED — the №621 lesson) and the
fail-closed baseline read.

### 2.2 The machine fact (№906) and its tamper traps

`gen_metrics.py --check` reads every factor from the live tree and refuses
below the floor:

```
OK: the №906 floor holds — TAINT_SATURATION 324241748923742258252688
(gate_042 floor 1000000000, only-up) — roster: 8 scenario rows, 6 live sources
```

The tamper traps (Traps 40–46, `--tamper-test`) prove the machine follows
the source: the lost roster row lowers the space; the dead anchor region is
a loud failure; the removed lane module / sanitizer entry ZERO it; the
removed sink name lowers it; the vacuum roster reads 0; the floor lowering
is refused. Verdict: `the traps lowered all` — GREEN.

## 3. What the fact GUARANTEES

- The corpus of taint-relevant programs the differential harness can reach
  is too large to memorize: the generator's reachable space exceeds 10⁹ by
  fourteen orders of magnitude. A green run over a handful of memorized
  shapes cannot masquerade as saturation — the space is the machine-checked
  argument.
- Every taint lane (secret/sql/html), every roster source and sink, and the
  sanitizer path are EXERCISED on both backends on every CI run, with the
  expected refusal CLASS asserted (a moved class is RED — the mutation-probe
  posture, not a snapshot).
- The saturation, once reached, cannot silently roll back: the space is
  floor-pinned only-up (gate_042), the triple footprint is pinned only-up
  (№905), and both are tamper-trapped.

## 4. The honest limits — what the fact does NOT guarantee

This section is the №588-class honest-limits record; the same content is
synced into `docs/limitations.md` and `docs/threat-model.md`.

1. **The space is NOT a correctness proof.** A ≥10⁹ reachable space argues
   the corpus cannot memorize the surface; it says nothing about the
   CORRECTNESS of the taint lattice itself. A systematically wrong rule
   reproduced across the whole space stays wrong — the guarantee is
   coverage, not soundness.
2. **The run is NOT the space.** No CI run executes 10⁹ (let alone 10²³)
   programs: a run exercises the 8 roster scenarios (7 distinct triples) —
   the space is the algebraic capacity of the generator, the run is its
   sampled witness. Both are pinned; neither is conflated with the other
   (the №894 honesty form: the count is CHECKED, not declared — and the
   check is arithmetic, not execution).
3. **The corpus is never tuned to green (№476).** A taint class can never
   become a `known-` line: a TW/VM divergence on a taint scenario is RED
   and un-pinnable. The roster grows only with the capability, never with
   the failures.
4. **The fuzzer does NOT replace a formal verification of the lattice**
   (the R38(5)-class honest frame). The lanes ride the typed label modules
   (`html_label.rs`, `sql_label.rs`, `secret_label.rs`) whose semantics are
   hand-audited; differential fuzzing can find backend divergences and
   roster-class regressions, it cannot prove the absence of lattice bugs.
5. **An external audit is NOT part of this report.** The external paid
   audit of the taint surface is the OWNER's gate (the R38(3) budget line,
   15–40 k$) — outside the pipeline's perimeter. This document equips the
   owner for that decision; it does not substitute it.
6. **The interprocedural depth stays bounded** (`METALOGOS_TAINT_DEPTH`,
   default 4) and the cross-module memory match stays a heuristic — the
   wave-81 saturation does not change those boundaries (they are the
   standing rows of `docs/threat-model.md` §Known Boundaries).

## 5. Reproducibility

```sh
# the machine fact (the exact number of §1) — deterministic, double-run:
python3 scripts/gen_metrics.py --check
python3 scripts/gen_metrics.py --check   # the byte-identical second run

# the tamper traps (the machine follows the source):
python3 scripts/gen_metrics.py --tamper-test

# the per-run lane (the both-backend agreement + the expected classes):
cargo test --test naryad_465_diff_fuzzer naryad_904_taint_lane_diff -- --nocapture

# the only-up triple ratchet (the LIVENESS + fail-closed guards):
cargo test --test naryad_905_taint_triples_ratchet -- --nocapture
```

## 6. The verdict

The saturation subphase fact (8F-2) holds: **TAINT_SATURATION = 324241748923742258252688
(≈ 3.24 × 10²³) ≥ 10⁹**, machine-checked over live anchors, tamper-trapped,
only-up floored; the run lane is both-backend pinned with the class
asserts; the honest limits are explicit. The next level of assurance — the
formal verification of the label lattice and the external audit — is the
OWNER's gate and is deliberately outside this pipeline.
