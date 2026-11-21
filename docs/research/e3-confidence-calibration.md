# E3 — the confidence calibration (№988, ADR-0200 §5)

- Status: measured (the round-2 E-line of the provable tier-lowering plan)
- Date: 2026-11-21
- Naryad: №988 (issue #2022, dispatch wave В99 — W-AC, №AC1)
- Harness: `tests/research_e1_e4.rs` (env-guarded — the tests pass as loud no-ops in CI and measure only under METALOGOS_RESEARCH_E1E4=1; the documented
  invocation `scripts/research/run_e1_e4.sh`); the machine-readable
  companion is `e3-confidence-calibration.json`, the raw run log is
  `e1_e4_raw_run.txt`.

## The question

`ADR-0200 §5`, the E3 row: the reliability curve of the model's
confidence on held-out data — deciles of the claimed confidence against
the actual accuracy per bin, the artifact being the table + the ECE.
Until E3 measured this, EVERY confidence was treated as `uncalibrated`
(§6.2) — the certificate literal stays `uncalibrated` after this
measurement too: the FLIP decision belongs to W5/W6, this naryad only
measures.

## The method

- The fixture: the same 400-text corpus (4 classes, seed `20261121`);
  the harness performs its OWN seeded stratified 70/30 split (LCG,
  seed `20261121 + 0xE3`): 280 train / 120 held-out. The model's
  INTERNAL 80/20 split (deterministic, №485/№489) is a separate split
  that feeds `train()`'s own accuracy figure — the two are independent
  and the artifact names both.
- The model: `dense(256,64,relu)+dense(64,4,softmax)`, 30 epochs, lr
  0.1 — the distill shape (E2's).
- The measurement: `forward()` on every held-out row gives the softmax
  probabilities; confidence = the max probability, correctness = argmax
  == the true class. Ten equal-width bins over [0,1); per bin — count,
  mean confidence, accuracy; ECE = Σ (n_b/N)·|acc_b − conf_b|. The
  majority baseline (the most frequent TRAIN class applied to held-out;
  the W2′ mandatory-baseline form) rides the same artifact — the
  balanced design ties 100/100/100/100 and the FIRST max wins the tie
  (the rule is recorded), so the baseline word is "alpha" and the
  baseline accuracy reads 0.25.

## The results

- n_train 280, n_held_out 120; the model's internal holdout accuracy
  0.9286; the harness held-out accuracy **0.8833**; the majority
  baseline **0.2500**.
- Non-empty bins:

| bin | count | mean confidence | accuracy |
|---|---|---|---|
| [0.2,0.3) | 100 | 0.2766 | 0.86 |
| [0.3,0.4) | 20 | 0.3089 | 1.00 |

- **ECE = 0.6013.**

## The honest reading

The model is massively UNDERCONFIDENT on this fixture: it answers
correctly 86–100 % of the time while claiming 0.28–0.31 confidence —
every bin's accuracy sits far ABOVE its mean confidence, which is what
drives the ECE to 0.60. Two machine facts follow. First, the
certificate's `eval.calibration` literal STAYS `uncalibrated` — the
measurement confirms the §6.2 norm rather than flipping it: a word like
"calibrated" would be a lie while the gap is this wide, and the flip is
W5/W6's decision anyway. Second, the W6 enablement gate (decision 5)
CANNOT read this model's confidence as a probability — a margin-gated
contour (`fallback_if confidence < t`) with t calibrated for a
well-calibrated model would fall back almost always; that is the
correct (safe) direction to be wrong in, and W5/W6 must either retrain
with a temperature path or set the thresholds against the measured
curve, not against an assumed calibration. The numbers measure THIS
machine, THIS fixture, THIS 30-epoch Dense shape — the honest-limits
register carries that boundary; the certificate literal is untouched.
