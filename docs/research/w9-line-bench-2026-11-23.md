# W9 line bench — the provable-lowering line's own overhead

Recorded: 2026-11-23. Machine: Intel(R) Xeon(R) Processor (a shared sandbox — the timings are indicative, not contractual). Profile: release; the numbers come only from the raw run logged next to this file (`w9_bench_raw_run.txt`); no planned or target numbers exist in this lane (ADR-0200 §5: «numbers measured only»). The demo pattern is examples/distill/line_demo.mlog (the direct train on labeled rows — deterministic, no LLM).

## Latency (wall time per single CLI invocation, ms)

| scenario | runs | min | median | p95 | max |
|---|---|---|---|---|---|
| `mlog attest verify --json` | 30 | 6.4 | 6.7 | 7.2 | 7.6 |
| `mlog distill status --json` | 30 | 4.9 | 5.1 | 5.3 | 5.5 |
| `mlog curriculum build --json (fresh db copy, held-out 4)` | 10 | 6.6 | 6.9 | 7.0 | 7.0 |
| `mlog run (train + save + inline verify, fresh store)` | 5 | 11.9 | 12.3 | 12.6 | 12.7 |

## Sizes (the store's attestation row)

| artifact | bytes |
|---|---|
| certificate (`cert_json`) | 905 |
| weights blob | 502 |

## Reading

The certificate + the ledger anchor ride the save in the same transaction (the W1 artifact rule), so the `mlog run` scenario IS the attested save path — no separate overhead line exists to measure. The E-series (E1–E4, `docs/research/`) measured the model-training and calibration economics; this bench measures the line's verification surface on top of them. No before/after baseline exists for the CLI verbs themselves — the verbs are new surface introduced by this line (honest note: no baseline).
