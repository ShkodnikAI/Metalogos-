# ADR-0217: Taint Saturation & the Formal-Guarantees Audit — the Reachable-Scenario Space as a Verified Artifact (the 8F-2 / C9 Subphase Record)

- Status: **Accepted** (the wave-81 acceptance — TAINT_SATURATION
  ≥ 10⁹ closed at 3.24×10²³ against the floor 10⁹, gate_042; the
  dispatch gh#1803 closed 2026-11-04 21:47Z with the acceptance table)
- Date: 2026-11-04
- Deciders: the OWNER (the R38 train — 0.44 = 8F-2 (C9, TAINT_SATURATION
  ≥ 10⁹ + the audit report); the wave-81 dispatch gh#1803; the «Заводи
  следующую волну» directive of 2026-11-04)
- Naryad: №903 (issue #1797, Wave 81 — W-J; ADR before the first code of
  the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38 — the C9 line
  «Формальные гарантии»); the R59 GET-only fact-check over live main
  `cbb3c64c` (2026-11-04)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.44 is this record's train slot),
  [ADR-0204](0204-portable-refusal-algebra.md) (the portable refusal
  algebra — the taint refusals this wave saturates are its carriers),
  [ADR-0198](0198-error-model.md) (the loud error model — a taint
  refusal is a loud compile-time refusal, never a silent pass),
  [ADR-0197](0197-vm-single-executor.md) (the VM is the only executor;
  TW is the reference oracle — the both-backend agreement this wave
  pins is that contract's fuzzing face), [ADR-0169](0169-stable-try-codes.md)
  (the stable refusal codes — the divergence classes this wave guards)

## §1. Context — the taint surface exists; its fuzzing saturation does not

Live main `cbb3c64c` carries a three-lane taint surface with machine
anchors: the secret lane (`src/secret_label.rs` — `secret_source_type:68`,
`is_private_labeled:74`; the loud refusal «print() refused: Secret» — the
p114 example, the n98 fixture), the sql lane (`src/sql_label.rs` —
`is_literal_sql:59`, `binding_sql_type:152`; SQL_DYNAMIC by position —
№831), and the html lane (`src/html_label.rs` — `sink_arg_is_untrusted:444`,
`TAINT_NESTING_MAX_DEPTH:138`; `escape_html` as the sanitizer per
`src/builtins_classification.rs:142`). The sinks are pinned by
`src/audit.rs:1872` (`SINK_FUNCTIONS`); the interp-side propagation walks
to depth 4 (`DEFAULT_TAINT_INTERP_MAX_DEPTH:4248`).

The C9 pillar («Формальные гарантии») rests today on three facts: the
differential fuzzer №465 (`tests/naryad_465_diff_fuzzer.rs`), the LIVENESS
ratchet №621, and the nesting-pairs 36/36 baseline (№766). None of the
three generates taint scenarios: the generator's forms are language-core
only (if/while/each/match + the memory group), so the taint surface is
INVISIBLE to the differential fuzzer — a backend divergence on a taint
scenario would be found by production, not by CI.

The №476 blocked-domain rule already forbids PINNING label/taint
divergences (a taint class can never become a `known-` line). That rule
makes the corpus honest, but it does not make it SATURATED: nothing
requires the fuzzer to exercise the taint surface at all.

The R38 train defines the 0.44 subphase fact as TAINT_SATURATION ≥ 10⁹
plus an audit report. The fact is a COUNT, not a share — this record
defines what it counts before any code lands (the №891 form: the ADR
fixes the definition, the naryad №906 implements the machine).

## §2. Decision — the fact is the reachable-scenario space; the evidence is two-component

**TAINT_SATURATION := the exact count of distinct taint-relevant programs
the wave-81 generator can emit — computed as an integer product over LIVE
anchors, never declared.**

The formula (implemented 1:1 by №906):

```
Space = ScenarioVariants × SequenceSpace
SequenceSpace = Σ_{k=0..B} F^k        (the statement sequences up to the budget B over F forms)
ScenarioVariants = T × L × S × K × Z  (scenario templates × lanes × sources × sinks × sanitizers)
```

Every factor is read from the live tree with a precise anchor:

| Factor | Meaning | Live anchor |
|---|---|---|
| T | the taint scenario templates of the wave-81 generator roster | the quoted roster region in `tests/naryad_465_diff_fuzzer.rs` (the №862 quoted-region form) |
| L | the live label lanes | the presence of `src/html_label.rs`, `src/sql_label.rs`, `src/secret_label.rs` (a removed lane module zeroes its factor) |
| S | the live sources reachable in the harness | the generator's source roster (the same quoted region) cross-checked against `src/secret_label.rs:68` / `src/html_label.rs:142,151` |
| K | the live sinks | `SINK_FUNCTIONS` (`src/audit.rs:1872`) + the sql entry points (№831) |
| Z | the live sanitizers | the classification of `escape_html` (`src/builtins_classification.rs:142`) |
| F, B | the generator's statement forms and budget | the live `FORM_*` roster (:249–254) and the budget constant in the same file |

The floor is **10⁹ only-up** — the wave-81 dispatch constant
(`goal_taint_saturation_min`, gate_042, the load_floors_strict form). A
lowered floor is refused loudly (the №776 discipline — a re-pin is an
OWNER decision, never a silent gate edit). If the live product is below
the floor, the honest repair is to RAISE the generator's reach (a real
capability raise — a wider budget/roster), never to loosen the formula.

The evidence is two-component, and both components are machine-checked:

1. **The space (the fact).** The exact product above — the guarantee that
   the corpus cannot memorize a tiny space: the reachable taint-relevant
   program space exceeds 10⁹ distinct programs.
2. **The run (the ratchet).** The per-run count of distinct taint triples
   (source, sink, lane) actually executed, pinned only-up in
   `scripts/ci/fuzz_taint_triples_baseline.txt` (the №766 nesting-pairs
   form) with a LIVENESS guard: zero generated taint scenarios is a RED
   run, never a pass (the №621 lesson — a sweep that silently executes
   nothing is a liar).

The both-backend agreement (№904) is the third, standing guarantee: every
generated taint scenario runs on TW and VM, and the outcomes (stdout or
the stable refusal class) must agree byte-for-byte. A taint divergence is
RED and un-pinnable — the №476 rule already refuses the shelter.

## §3. The parents are consumed by reference — what this record does NOT redefine

- The label lattice, the lane semantics, the sink sets, the sanitizer
  classification — unchanged. This wave SATURATES the surface; it does
  not move it (the 1:1 migration discipline, §6 of ADR-0210's form).
- The differential fuzzer's harness, minimization, and corpus format
  (№465/№479) — extended, not replaced; the №476 blocked-domain rule is
  the standing no-shelter machine.
- The refusal classes — the frozen vocabulary (№385/ADR-0169) is not
  extended; the generated scenarios reuse the existing classes
  (SECRET_LEAK-family refusals, SQL_DYNAMIC, the html egress refusals).
- gen_metrics/gate discipline — the established machine-fact pattern
  (№735: the fact rides with its checker; №841/№862/№894: the roster +
  tamper traps + the only-up floor).

## §4. The consumption contracts — what №904–№908 land

1. **№904 — the taint lane in the fuzzer.** The generator emits taint
   scenarios over the live lanes reachable in its harness (the secret and
   sql lanes in the №465 harness; the html lane in the №585 route-body
   fuzzer where `respond*` shapes live); the per-run triple counter rides
   the report; the mutation probes prove a gate bypass turns red.
2. **№905 — the coverage ratchet.** The triples baseline (only-up) + the
   LIVENESS guard + the fail-closed baseline read.
3. **№906 — the machine fact.** `taint_saturation()` in gen_metrics.py +
   gate_042 (the floor 10⁹, the tamper traps — the vacuous roster is a
   failure, the dead generator is 0, the floor lowering is refused) + the
   GENERATED rows in `docs/metrics.md` and `docs/PLAN-SUMMARY.md`.
4. **№907 — the audit report.** `docs/audits/taint-saturation-2026-11.md`
   — the methodology, the live evidence tables, the honest limits (§5);
   the limitations/threat-model sync. This is the «отчёт аудита» artifact
   of the train line. It is INTERNAL: the external paid audit is the
   owner's gate (R38(3), the 15–40 k$ budget reserved — procurement is
   an owner act, outside the conveyor).
5. **№908 — the ride.** «The floor follows the fact»: the full surface-gate
   verification on the wave merge chain; NO DRIFT expected (the wave is
   test-only + the machine + docs — no registry rows grow).

## §5. Honest limits — what TAINT_SATURATION is NOT

- **Not a correctness proof.** A ≥10⁹ reachable space says the corpus
  cannot exhaust or memorize the taint scenario space; it does NOT prove
  the absence of taint defects. The correctness evidence is the
  both-backend agreement on every generated scenario plus the standing
  leak suite — not the space count.
- **Not an execution count.** No CI run executes 10⁹ programs; the space
  is a structural property of the generator, computed exactly from live
  anchors. Claiming it as "executed" would be a lie — the audit report
  (№907) states this in the honest-limits section.
- **Not tuned to green.** The corpus and the roster are never adjusted to
  make a run pass (the №476 preamble rule); a shrinking surface (a lane
  or sink removed) lowers the fact below the floor — the gate working,
  not a pass.
- **Not the external audit.** The 0.44 train line says «+ отчёт аудита» —
  that artifact is №907 (internal, machine-verifiable). The external
  institutional audit stays the owner's gate with the reserved budget.

## §6. The migration discipline — 1:1, byte-identical surfaces

The wave touches test files, the metrics machine, and docs only. The
grammar, the builtin registry, the golden outputs, the runtime modules —
byte-identical (the №687 counter reports zero surface growth; the №895
ride form verifies no drift).

## §7. Acceptance

- ADR-0217 Proposed → Accepted at the wave-81 acceptance (the dispatch
  gh#1803 records the verdict).
- The gate 0.44: `gen_metrics --check` green with TAINT_SATURATION ≥ 10⁹
  on the wave merge chain; the tamper traps alive; the triples baseline
  only-up; the both-backend taint agreement green on every generated
  scenario.
- The audit report present with reproducible numbers (every number is
  regenerable by the machine or a real run — no prose numbers).
