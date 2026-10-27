# The Distillation Experiments (№823, E1–E4): the Measured Baseline for the W4–W6 Decisions

- Meta: naryad №823 (issue #1600, wave 68, P2, class [research]); the
  ADR-0200 «provable tier-lowering» line (the E1–E4 roadmap row).
- Date: 2026-10-28. Binary: the release build of the tag commit
  v0.34.0 (`6d84951` + the wave-68 hygiene merges; the `embed` runs
  need the `vec` feature, see E1).
- Rule held: measurements only — no product code changed; every number
  below is measured by the checked-in scripts, the raw streams are the
  CSVs beside this document.
- Hardware: Linux 5.10.134 x86_64, Intel(R) Xeon(R), 2 cores, the
  office sandbox. The absolute numbers are sandbox-relative; the
  RATIOS and the scaling shapes are the transferable results.

## The measurement harness

All four experiments drive the RELEASE binary (`cargo build --release
-p metalogos-server --bin mlog`) with generated .mlog programs; the
timing happens INSIDE the program (`now()` deltas, f64 epoch seconds —
SystemTime, non-monotonic: fine for the sub-10 s windows here, recorded
as the honest limitation), the memory via the child's rusage
(`os.wait4`, `ru_maxrss`). The scripts:

- `scripts/research/e1_embed_speed.py` → `e1_embed_speed.csv`
- `scripts/research/e2_train_curve.py` → `e2_train_curve.csv`
- `scripts/research/e3_calibration.py` → `e3_calibration.csv` + the
  per-seed raw streams `e3_calibration_seedN_raw.csv`
- `scripts/research/e4_teacher_agreement.py` →
  `e4_teacher_agreement.csv`

Reproduce:

```
cargo build --release -p metalogos-server --bin mlog --features metalogos/vec
python3 scripts/research/e1_embed_speed.py --reps 7
cargo build --release -p metalogos-server --bin mlog
python3 scripts/research/e2_train_curve.py --reps 3
python3 scripts/research/e3_calibration.py --seeds 3
python3 scripts/research/e4_teacher_agreement.py --n 60
```

(The `embed` builtin is `#[cfg(feature = "vec")]` — the E1 binary
carries the feature; E2–E4 need no features. The E1 harness raises
`METALOGOS_CONTOUR_BUDGET=100000` consciously — the default 64/tick is
the production posture of the №546 seam, a 10k-line loop is a
benchmark, not a request shape.)

## E1 — the `embed` speed (the SSOT TF-IDF backend)

The default backend is the vocab-growing `TfidfEmbedding` (the №546
SSOT seam: the same manager the distill contour and the builtin share;
dim = max(vocabulary, 256)). Two corpus shapes are measured — the
steady-state (a fixed ~120-token vocabulary; the dim stays at its 256
floor) and the vocab-growth (every line carries a unique token; the
vocabulary — and with it the vector dim — grows with the corpus). Three
arms per size: the embed loop, the plain loop (`len` only), and the
alloc-matched loop (constructs a fresh 256-Float list per iteration —
the embed result's allocation shape). p50 over 7 reps, in-program ms:

| shape | n | embed | plain | alloc-matched | embed−plain |
|---|---|---|---|---|---|
| steady | 1k | 62.6 | 51.3 | 81.7 | **11.4 µs/line** |
| steady | 10k | 5668.2 | 5378.5 | 6073.9 | **29.0 µs/line** |
| grow | 1k | 63.9 | 51.0 | ~82 | **12.5 µs/line** |
| grow | 10k | 6451.5 | 5373.1 | 6082.2 | **107.8 µs/line** |

Findings:

1. **The steady-state embed is cheap**: ~11–12 µs/line at both sizes —
   the tokenize+hash+TF-IDF compute is O(tokens) per line, constant.
   The 1k→10k growth of the DELTA (11.4 → 29.0 µs/line) is dominated by
   the VM loop artifact (see the honest note below), not by the embed
   compute itself.
2. **The vocab growth is the real cost driver**: on the distinct-token
   corpus the per-line cost rises to 107.8 µs/line at 10k lines
   (~10.3k distinct tokens → dim ~10.3k → the O(dim) vector alloc +
   normalize per call, ~8.6× the 1k figure). For W4–W6: a distill
   corpus with unbounded vocabulary makes embedding superlinear per
   line; either bound the corpus vocabulary or switch the feature
   backend to the fixed-dim hash form (`TfidfHashEmbedding`, №504) —
   the switch is a decision lane, this measurement is its input.
3. The alloc-matched arm OVERSHOOTS (the deltas are negative at three
   of four configs): constructing a 256-element list literal inside the
   TW loop costs MORE than the whole embed call. Honest reading: the
   interpreter's per-iteration list construction (~80 µs at 256
   elements) is heavier than the native Rust call packaging the same
   result — a VM property, not an embed property; kept in the CSV as
   measured.

Honest note (the loop artifact): the VM evaluates the `texts[i]` index
per iteration with an O(n)-ish cost on a big stored list (the plain
baseline grows from 51 µs/iter at n=1k to 538 µs/iter at n=10k — the
per-iteration cost scales with the corpus). The deltas carry a
second-order contamination from that artifact; the vocab-growth signal
(8.6×) is far above it, the steady-state figure (~11 µs/line) is the
lower bound. The `each`-over-literal form was measured first and is
WORSE (the iterable expression re-evaluates per iteration — O(n²)
total); the while+index form is the reported one. Both raw series are
in the harness history (the CSV carries the while+index run).

## E2 — the `reflex_train` time/memory curve

A Dense head (input 16, dense(32, relu), dense(C, softmax), seed 42)
trains on N synthetic linearly-separable rows for 200 epochs (the
dispatch constants: lr 0.1, the `accuracy` metric). The grid C ∈
{2, 5, 10} × N ∈ {50..800}, p50 of 3 reps:

| C | N=50 | N=100 | N=200 | N=400 | N=800 | (ms/epoch) |
|---|---|---|---|---|---|---|
| 2 | 31.9 | 64.8 | 128.4 | 256.8 | 515.0 | 0.16 → 2.58 |
| 5 | 35.0 | 71.1 | 141.1 | 282.0 | 571.0 | 0.18 → 2.86 |
| 10 | 41.2 | 83.7 | 166.1 | 331.9 | 666.1 | 0.21 → 3.33 |

Memory (max RSS per run): ~17.7 MB at N=50 → ~43 MB at N=800 (the
baseline program with the largest dataset BUILT but not trained sits at
~42 MB — the trained runs' RSS is dataset-dominated at these sizes).

Findings for W4–W6:

1. **The training time is LINEAR in N** (~3.2–4.2 µs per example per
   epoch across the grid) and mildly superlinear in C — the background
   training thread (the №489/№495 posture) can afford the serve-scale
   corpora (800 examples × 10 classes = 0.67 s per 200-epoch run) —
   the in-request blocking is the thing to avoid, not the training
   itself.
2. **The convergence budget is C-sensitive**: under the fixed 200-epoch
   budget the head fully solves C=2 (acc 1.0), mostly solves C=5
   (~0.80 at N=200), and partially solves C=10 (~0.35) on the compact
   offset scale. The W4 curriculum design must either raise the epoch
   budget with C or accept the per-class quality floor — the measured
   property of the current Dense trainer, not a defect claim.
3. The loss figures are reported verbatim in the CSV; the C=10 rows
   carry an identical loss across N (34.5388 at the wide-offset
   pre-fix shape) — the loss NORMALIZATION semantics (sum vs mean over
   rows) deserve a look in a follow-up lane; E2's object is the
   timing/memory, not the loss semantics.

## E3 — the confidence calibration (reliability + ECE)

Procedural ground truth (a seeded linear score + logistic noise, 2
classes, 2 features) — NO LLM anywhere in the loop: the held-out set
is clean of teacher provenance by construction (the E3 requirement).
Train 400 / held-out 200, disjoint seeded streams, 3 seeds; the
Dense head predicts every held-out row, the top-variant confidence
rides the `confidence()` builtin:

| seed | held-out top-label acc | ECE (10 bins) |
|---|---|---|
| 1 | 0.875 | 0.0456 |
| 2 | 0.890 | 0.0434 |
| 3 | 0.875 | 0.0436 |

The reliability tables (in `e3_calibration.csv`) are
textbook-shaped: the top bin (conf 0.96–0.97) carries acc 0.99–1.00
(well calibrated where it matters); the mid-bins (0.5–0.8) are noisy
at n=11–24 (acc 0.39–0.82 vs conf 0.54–0.75).

Findings for W4–W6:

1. **The softmax head is broadly calibrated on this shape**: ECE ≈
   0.044 mean over the seeds — the confidence CAN be consumed as a
   probability on clean 2-class procedural data, which is the number
   the W1 certificate and the W6 enablement gate lean on.
2. **The calibration is shape-specific, not universal**: this is a
   2-class/2-feature/procedural result. The W4 curriculum and the W6
   gate must RE-MEASURE per real corpus (the harness runs as-is on any
   labeled held-out stream); the mid-bin instability at small n says
   the gate thresholds must sit in the well-populated top bins.
3. The naryad's precondition («everywhere should read `uncalibrated`
   until E3») now has its measured counter-value: `calibrated (ECE ≈
   0.044, 2-class procedural, 3 seeds)` — the label upgrade on the
   real corpora is W4's call with this harness.

## E4 — the teacher's baseline agreement

The teacher is a REAL LLM-API path with a DETERMINISTIC rule labeler
behind it: a local HTTP server implements the OpenAI
`/chat/completions` contract and labels by the same rule the ground
truth comes from, except a KNOWN error pattern (`tricky` texts get the
flipped label) — the teacher quality is a measured number, not a
tautology. Network-free (127.0.0.1). The serve program wires a
learnable pattern (`distill_to: TeacherHead`, `distill_after: 100000`
— TEACHING mode throughout) behind GET /classify; the driver sends the
60-text corpus, the №495 hub persists the examples into
`distill_samples`, the driver reads the db and computes:

| measure | value |
|---|---|
| teacher vs ground truth | **57/60 = 0.9500** |
| teacher vs the majority baseline («neg») | 32/60 = 0.5333 |
| the majority baseline vs ground truth | 35/60 = 0.5833 |
| persisted distill_samples rows | 60/60 |

Findings for W4–W6:

1. **The end-to-end teacher→db→agreement lane works**: 60/60 examples
   landed in `distill_samples` through the serve hub (the №495
   persistence), the label extraction is deterministic, the agreement
   machinery is reproducible with ONE command.
2. **The teacher beats the majority baseline where it should**: 0.95
   vs 0.58 accuracy — on this corpus the teacher labels carry real
   signal over the trivial baseline. The W4 decision («trust the
   teacher above X») now has the measurement SHAPE; the real-teacher
   numbers ride the same harness with the corpus swapped.
3. The corpus is deliberately small (60) and rule-labeled; the honest
   limit: E4 measures the MACHINERY and a deterministic teacher, not
   any production LLM's quality — the real-teacher run needs the
   owner's `llm {}` keys (out of the sandbox), the harness accepts it
   unchanged.

Honest finding recorded (the serve router gap): the route lane's
learnable LLM call resolves the LEGACY env backend — the per-request
interpreter carries no smart router, so the program-level `llm {}`
declaration is invisible on the serve route lanes (the global bridge
covers the `call_llm` BUILTIN, not the learnable method's
per-interpreter guard). E4 drives the teacher through the designed env
lane (`METALOGOS_LLM_PROVIDER=openai` + `METALOGOS_API_KEY` +
`METALOGOS_OPENAI_BASE_URL` → the local teacher) — the measurement is
unaffected, and the gap itself is a candidate for a follow-up naryad
(the class: the №495 `clone_definitions_into` surface did not grow the
router field; the TW run lane works, the serve route lane does not).

## The W4–W6 conclusion set (one paragraph each)

- **W4 (the curriculum)**: the training cost is linear and small (E2);
  the binding constraint is the CONVERGENCE budget per class count and
  the corpus vocabulary (E1's growth finding) — the curriculum should
  bound the corpus vocabulary (or pin the hash backend) BEFORE scaling
  the class count.
- **W5 (the frozen-model recovery)**: the memory curve (E2) is
  dataset-dominated and bounded at these scales — the recovery checks
  can afford full re-trains up to the serve-scale corpora; no
  incremental-weights machinery is required by the numbers.
- **W6 (the enablement gate)**: the confidence is consumable (E3,
  ECE ≈ 0.044 on the clean shape) but shape-specific — the gate reads
  the per-corpus ECE from THIS harness as its input, with the
  thresholds placed in the well-populated top confidence bins.
- **The teacher trust lane (E4)**: the agreement harness is the
  reusable instrument (teacher vs truth, teacher vs majority, baseline
  vs truth on any labeled corpus); the W4 «trust the teacher above X»
  decision takes its X from a real-teacher run of the same harness.
