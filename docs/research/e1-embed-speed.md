# E1 — the `embed` speed (№988, ADR-0200 §5)

- Status: measured (the round-2 E-line of the provable tier-lowering plan)
- Date: 2026-11-21
- Naryad: №988 (issue #2022, dispatch wave В99 — W-AC, №AC1)
- Harness: `tests/research_e1_e4.rs` (env-guarded — the tests pass as loud no-ops in CI and measure only under METALOGOS_RESEARCH_E1E4=1; the documented
  invocation `scripts/research/run_e1_e4.sh` — release profile, one test
  thread); the machine-readable companion is `e1-embed-speed.json` next
  to this file, the raw run log is `e1_e4_raw_run.txt`.

## The question

`ADR-0200 §5`, the E1 row: the `embed` throughput (examples per second)
across batch sizes on a standard fixture set, with the machine facts
(hardware, versions, seeds) recorded. E1 feeds W4 (`mlog curriculum
build` depends on E1) and bounds the embed-side cost of every later W
step that touches the embedding lane.

## The method

- The fixture: 400 synthetic texts, 4 balanced classes (100 per class),
  5 words per text, drawn by a fixed 64-bit LCG (Knuth MMIX constants,
  seed `20261121`) whose code IS the recorded generator — no dependency
  RNG is touched, so a `rand` patch-bump cannot move the fixture (the
  №987 lesson: the seed is fixed in the result record, the tests pin no
  generator sequences). The word pools (24 nouns per class + 8 template
  verbs) keep the vocabulary below 256, so the in-process TF-IDF
  dimension is the constant 256 for every text.
- The backend: the `embed` builtin's default lane — the in-process
  TF-IDF manager (`METALOGOS_EMBEDDING_PROVIDER=tfidf`, explicit in the
  run), `EmbeddingManager::embed` — the SSOT path the builtin itself
  calls.
- The measurement: per batch size (1, 8, 32, 128) × 3 repetitions, each
  repetition embedding the whole corpus cold with a FRESH manager (the
  TF-IDF vocabulary build is part of the measured cost; the call
  sequence is identical every time). Wall-clock via `Instant`;
  throughput = rows / seconds; the median of 3 is reported.
- Environment: `rustc 1.100.0 (0ef1bc386 2026-11-09)`, the pinned
  toolchain of `rust-toolchain.toml`, 2 CPUs, Intel(R) Xeon(R)
  Processor, release profile — the exact facts also ride the JSON.

## The results

| batch size | median examples/s | reps (examples/s) | dim |
|---|---|---|---|
| 1 | 437 896 | 407 454 / 437 896 / 439 689 | 256 |
| 8 | 438 408 | 423 918 / 439 701 / 438 408 | 256 |
| 32 | 446 011 | 445 699 / 446 238 / 446 011 | 256 |
| 128 | 446 160 | 434 180 / 446 160 / 447 029 | 256 |

## The honest reading

The throughput is size-INVARIANT across batch sizes (±2 % across all 12
repetitions) because the lane has **no batching primitive**: `embed()`
takes one text per call and the "batch" is only a grouping of identical
calls. That is a machine fact W4/W5 must design against: any batch-side
speedup first needs a batch-capable embedding path — today the lane's
~440 k examples/s (256-dim, 5-word texts, cold manager) is the per-call
cost, and it is already far above every contour's need (the whole 400-row
fixture embeds in under a millisecond at this scale). The number is a
measure of THIS machine and THIS fixture shape — not a universal
statement (the honest-limits register; the JSON carries the hardware).
