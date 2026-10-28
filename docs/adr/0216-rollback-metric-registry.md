# ADR-0216: The `rollback_if` Metric Is a Registry Name — the Gate Reads the Metric's Own Value and Carries the Majority Baseline (the W2′ Grammar Semantics)

- Status: **Accepted** (the наряд-mandated ADR-запись — the W2′ roadmap
  line of the provable tier-lowering plan; the wave-68 owner trigger
  27.10 «зарегистрировать №825/№826 в реестре, затем издать», the
  execution directive 28.10)
- Date: 2026-10-28 (the implementation record)
- Deciders: the OWNER (the W2′ decisions 1–3, §13.12, verbatim in
  gh#1599); the machine executes the record
- Naryad: №826 (issue #1609, Волна 68)
- Source: the W2′ line of `naryads/W1_reflex_attestation_and_roadmap_v2.md`
  (§Дорожная карта); the owner's decisions (2) "the metrics are a Rust
  registry only" and (3) the grammar semantics record requirement
- Related: [ADR-0200](0200-provable-lowering.md) (the provable tier
  lowering — the parent line), №485 (the distill-gate majority baseline —
  the precedent this record extends), №179 Block 3 (the
  `METRIC_REGISTRY` pattern, ADR-0114 addendum), №545 (the metric math
  SSOT in `metalogos-reflex`), №375/ADR-0112 (the real battery + the
  mock stub contract), №809 (the builtin tier gate — untouched here)

**Booking note (the number).** 0201–0215 are RESERVED for the Phase 8
line (the office canon, `language_strategy_plan_v2.md` §23, the R36
renumeration: 0201 = the Phase 8 through-line, 0202 = ModelPassport, …
0215 = the institutional contour; the rule "an ADR is published before
the first code of its subphase"). This record is NOT a Phase 8 ADR, so
it takes the next free number after the reservation — **0216** — by the
волна-17 precedent (the numbering walked past the 0154–0161 booking the
same way). The gap is intentional; no Phase 8 number was consumed.

## Context

The W2′ line needs the tier-lowering gate to read HONEST quality
signals. Before this record:

- `METRIC_REGISTRY` (№179 Block 3) carried exactly one metric,
  `accuracy`; the grammar pinned the `rollback_if` metric as the
  LITERAL `accuracy` (`src/grammar.pest`), so a gating rule could not
  name another metric at all.
- The mutate gate compared the battery accuracy against the threshold
  and reported no reference point: on a skewed battery a degenerate
  keep/rollback verdict looks identical to an informed one. The distill
  gate solved this in №485 with the majority baseline ("baseline +
  margin"); the mutate gate had no equivalent.

## Decision

1. **The registry grows to three metrics** — `accuracy`,
   `macro_f1`, `min_class_recall` — in the SAME `MetricSpec` form (no
   new `Value` variants). The math lives in
   `metalogos-reflex/src/nn/metric.rs` (the №545 SSOT): the class-space
   bodies (`macro_f1_labels`, `min_class_recall_labels`) are the ONE
   implementation; the `compute_*` adapters (the registry signature —
   probability vectors argmaxed to classes) route through them, and the
   mutate gate converts its held-out label pairs to the same class
   space. Edge semantics (pinned by contract tests): `macro_f1` skips a
   class present in neither truth nor predictions, counts a
   predictions-only class at F1 = 0; `min_class_recall` skips
   truth-less classes (undefined, not zero); empty or unequal evidence
   → 0.0, never invented.

2. **The grammar accepts a metric NAME.** `mutate_rollback` binds
   `IDENT` instead of the `"accuracy"` literal. The old form
   `rollback_if: accuracy OP x` stays valid — `accuracy` is a
   registered name, so every existing example and the golden corpus
   parse and output identically (pinned). The name is resolved against
   `METRIC_REGISTRY` at the SEMANTIC CHECK: an unregistered name is a
   loud refusal (`mutate: unknown metric '...'. Registered metrics: ...`
   — the `reflex_train` convention), never a silent fallback. The
   refusal covers both lanes by construction: the semantic check runs
   on the AST before any backend compiles it.

3. **The gate reads the metric's own value and carries the majority
   baseline.** The held-out (expected, actual) pairs are kept in the
   battery report; a backend error is the residual `<no-answer>` class
   (an unanswered task IS a failure for every metric). The gate value
   and the baseline route through the ONE registry compute (one-hot
   rows; the argmax recovers the class). The **baseline surface
   choice**: computed ON THE SPOT from the same held-out labels — not a
   grammar field. A written baseline field can go stale silently; a
   computed one cannot. The baseline is the metric of the
   always-majority predictor over the same truth (ties → the smallest
   label — deterministic), the №485 pattern brought to the mutate gate.

4. **Refusal semantics.** A gating rule with NO held-out evidence has
   NO baseline → a LOUD refusal (`mutate: rollback gate '...' has no
   majority baseline — no held-out evidence ...`), on both lanes — a
   self-modifying system does not gate on nothing. This tightens the
   №375 held-out-0 posture for the GATED case only: a mutation with no
   rollback condition is still measured, reported, and kept (the №375
   contract unchanged), and its 0.0-accuracy warning stays.

5. **Message surface (byte-parity discipline).** Both lanes format the
   verdict through ONE builder. For the default `accuracy` metric the
   string is byte-identical to the pre-№826 formats — the №375 mock
   pins and the p2 golden hold without edits. The gated REAL decision
   gains the `(majority baseline METRIC=V)` segment (the accompaniment
   requirement). The MOCK stub stays 0.95 for every metric — the
   mechanism exercise, not a quality signal (ADR-0112) — and the mock
   message carries NO baseline segment (there is no evidence surface in
   mock mode; fabricating one would be a silent lie).

## Alternatives considered

- **A grammar `baseline:` field** (the author writes the expected
  majority baseline): rejected — the value would be hand-written,
  unverified, and stale-prone; the computed baseline is derived from
  the same evidence as the verdict and cannot disagree with it.
- **Balanced accuracy instead of the baseline** (the №485 alternative):
  rejected for the same reason №485 rejected it — the degenerate
  majority predictor CAN clear a balanced-accuracy gate; it cannot
  clear baseline + an honest read of the same metric.
- **Extending the distill holdout gate to named metrics in this
  наряд**: out of scope — the distill gate already carries the majority
  baseline (№485) and its metric surface stays `accuracy`; extending it
  belongs to the W4+ line where the attest certificate consumes
  `eval.metric` (№825 coordination point).

## Consequences

- The gate verdicts on skewed batteries are now READABLE: the log shows
  the metric, its value, and what the always-majority predictor would
  have scored — a keep below the baseline is visible as such.
- The registry is the single extension point: a new gating metric is a
  `MetricSpec` row + the SSOT math, with zero grammar change (the №179
  posture holds).
- The residual `<no-answer>` class makes unanswered held-out tasks
  visible in every named metric (macro F1 gains a 0-F1 class; the
  minimum recall drops through the missed truth classes) — no metric
  can look good by ignoring the tasks the backend failed to answer.
- The `MutateDecl`/`MutateData` surfaces grew one field
  (`rollback_metric`); the bytecode payload is additive (the №780
  enumeration test updated).
- REFERENCE §5.15 documents the name form, the registered set, the
  baseline segment, and the refusal semantics.

## Verification (the наряд contract, `tests/naryad_826_metric_registry.rs`)

RED-first (форма №772): 9 of 11 tests fail on the unpatched main (the
registry exposes only `accuracy`; named metrics fail at parse; no
baseline surface), the two preservation pins (the old form, the p2
golden) pass there by construction. After the patch: 11/11 green, the
№375 mock/real suites, №148/№510 rollback-op pins, the golden corpus,
and `readme_consistency` stay green; `macro_f1`/`min_class_recall` are
pinned on hand vectors through the public registry surface.
