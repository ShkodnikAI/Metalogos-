# E-artifact: the edge inventory — the release binary v0.40.1, measured 2026-11-24

**Naryad:** №1001 (issue #2061, волна В105 — W-AI; the Phase 7 «Edge deepening + verifier» wave 1). **Companion ADR:** ADR-0224 (§1 consumes these numbers, §3 consumes the gaps). **Form:** the E-artifacts of the closed line (docs/research/w9-line-bench-2026-11-23.md — the numbers only from the recorded run). **The measurement tool:** `scripts/edge_inventory_measure.py` (rides this PR — the release download, the SHA256 verification, the size/startup/RSS measurement).

## The source artifact (the provenance chain, machine-verified)

| Fact | Value | Link/proof |
|---|---|---|
| Release | v0.40.1, created 2026-11-21T20:25:47Z, commit `245fe99610` | the release page; the asset `mlog-linux-x86_64` |
| Asset size | **37 318 352 bytes (35.59 MiB)** | the asset metadata + the local download (byte-identical) |
| SHA256 | `96d2db1bca793ec71fe6927b507ca0a1…` — **verified against the release `SHA256SUMS`** | the verification in the measurement run (the tool asserts the match) |
| Build provenance | rustc/cargo **1.100.0 (2026-11-05)**, LLVM 23.1.5, the runner image **ubuntu24 20261116.133.1**, `cargo build --release --locked -p metalogos-server --bin mlog` | the release `BUILD-INFO.txt` (quoted verbatim in the run output) |
| SBOM | CycloneDX, 0.4 MiB | the release asset `mlog-linux-x86_64.cdx.json` |

## The measured profile (zero target numbers — only what ran)

| Metric | Value | Platform (honest) | Method |
|---|---|---|---|
| Startup, `--help`, exit 0 | **median 0.003 s** (5 runs: 0.003/0.003/0.003/0.003/0.005) | the executor's sandbox: linux x86_64, kernel 5.10.134-013.15.kangaroo.al8, glibc 2.41 — **NOT a CI-runner run** | wall-clock `perf_counter` over 5 process spawns |
| Peak RSS, `--help` | **25.0 MiB** (getrusage `RUSAGE_CHILDREN` max) | the same sandbox | `resource.getrusage` after the runs |
| Binary size | **37 318 352 B** | the release asset (platform-independent fact) | the asset metadata + the downloaded bytes |

**The startup number is the CLI floor** (`--help` — the arg-parse path), not an interpreted-program run; a workload-level startup measurement does not exist yet (ADR-0224 §3.4).

## What works unchanged (the machine facts)

The artifact runs on linux x86_64 with glibc 2.41 (5/5 spawns exit 0); the provenance chain (SHA256SUMS → BUILD-INFO → SBOM) is machine-checkable by a 30-line script without repo access to CI internals.

## The honest gaps («нет прогона» — named, not silent)

1. **No other platform exists**: the releases carry linux-x86_64 only — no darwin / windows / arm64 artifacts, no runs on any of them.
2. **No CI-runner startup/RSS artifact**: the 0.003 s / 25.0 MiB above are the executor's sandbox numbers (the same linux-x86_64 class, a different machine); the runner-side measurement was never taken.
3. **No CI gates on size/startup/RSS**: nothing fails when these numbers regress (ADR-0224 §3.1).
4. **No recurring bench**: the verifier numbers (the `attest verify` median 6.7 ms — the w9 bench, 23.11) are a single recorded session, not a re-run lane (ADR-0224 §3.2).
5. **No workload-level startup**: the CLI floor only (ADR-0224 §3.4).

## The verdict posture

The inventory supports the ADR-0224 §1 claim (the provenance chain is live and machine-verifiable; one platform, one artifact, one session) and §3's absent-list. Nothing here is a budget or a promise — the owner fixes both, verbatim, before В106.
