# ADR-0224: Phase 7 «Edge deepening + verifier» — the line ADR (the edge profile as a measured surface + the verifier depth)

**Status:** Accepted (the owner's verbatim 24.11 — registered verbatim in docs/PLAN-SUMMARY.md Wave 33, naryad №1003 gh#2066; the В106 gate passed; the ADR-before-code rule — the ADR-0205 precedent)
**Date:** 2026-11-24
**Naryad:** №1001 (issue #2061, волна В105 — W-AI, the line-forming wave 1 of Phase 7)
**Depends on:** ADR-0200 (the verifier machine this line re-uses — attest/e2e/bench against the compiled binary), §13 ADR-0200 (the provable-lowering line closed W1–W9 + E1–E4, squash `6804aad37`), №998 (the W9 bench artifacts: docs/research/w9-line-bench-2026-11-23.md — the verify median 6.7 ms), the release provenance chain (SHA256SUMS / BUILD-INFO.txt / SBOM per release), №996 (the REFERENCE/README doc surface)
**Blocks:** the Phase 7 waves (В106+ — the slots fixed in §5 only after the owner's verbatim)

## 1. Context

The public plan digest (docs/PLAN-SUMMARY.md, «List of phases») names three unexecuted phases; the В105 dispatch (gh#2060) selected **Phase 7 «Edge deepening + verifier»** as the first non-blocked one: phase 5 «Embodied, sim-only» is behind the GPU-budget gate (the owner's input, not open), phase 6 «Spatial / XR» has no support in the pre-study queue or the watchlist (WL-01…WL-07 contain no XR interest). Phase 7 has a direct junction to the just-closed line: the verifier machine (attestation, e2e, bench) re-uses unchanged.

The machine facts this ADR builds on (measured 2026-11-24, the full numbers and the links in docs/research/edge-inventory-2026-11-24.md):

- The CI-built release artifact exists per release with a machine-verifiable provenance chain: `mlog-linux-x86_64` **37 318 352 bytes (35.59 MiB)** (the release v0.40.1 asset, created 2026-11-21), SHA256 verified against the release `SHA256SUMS`, `BUILD-INFO.txt` carries rustc 1.100.0 / LLVM 23.1.5 / the runner image ubuntu24 20261116.133.1 / `cargo build --release --locked -p metalogos-server --bin mlog`, plus the CycloneDX SBOM (`.cdx.json`).
- Startup (median of 5 runs, `--help`, exit 0): **0.003 s**; peak RSS (getrusage children max): **25.0 MiB** — measured on the executor's sandbox (linux x86_64, kernel 5.10.134, glibc 2.41). **Honest label: NOT a CI-runner run** — the runner numbers do not exist as artifacts (named gap, §3).
- The releases carry **linux-x86_64 only** — no darwin / windows / arm64 artifacts (honest: no run exists on any other platform).

## 2. Goals and non-goals

Goals: (a) the edge profile — the binary size, the startup, the memory, the constrained-platform readiness — becomes a **measured surface**: every number a run link, every gap named; (b) the verifier depth over the edge binary: which W1–W9 gates already cover it (§3), which are absent — named, not silent; (c) the honesty words: **an edge claim exists only with a run on the named platform** — no platform is claimed ready without its own run.

Non-goals (this wave): no product-code edge optimizations (the code gate is the owner's verbatim on this ADR); no new platform builds; no target/budget numbers — the budgets, if any, are fixed by the owner in §5 after the verbatim, not invented here.

## 3. The verifier junction (what covers the edge binary today — and what does not)

Already covers (measured, linked):

- The release provenance: SHA256SUMS + BUILD-INFO + SBOM per release — the artifact-level attestation chain (verified by the download of v0.40.1; the script is the inventory's companion tool).
- The e2e/attest/bench machine (№998) runs **against the compiled binary** — 10/10 RED-able; `attest verify` median 6.7 ms (the w9 bench, 23.11, the recorded run link in the artifact).

Absent (named, not silent):

1. No CI gate on size / startup / RSS — the numbers exist only as one-off measurements, nothing fails when they regress.
2. No recurring bench gate — the 6.7 ms verify median is a single 23.11 record, not a re-run lane.
3. No cross-platform builds or verification — one artifact family, one platform.
4. No workload-level startup measurement (the 0.003 s is the CLI floor `--help`, not an interpreted-program run).

The reuse posture: the W1–W9 harness (attest/e2e/bench) carries the edge-profile checks in the W2+ slots unchanged — no new machine is introduced by this ADR; the slots decide which checks graduate from measurement to gate.

## 4. The honesty words (line-wide)

1. An edge number without a run link does not exist (the «numbers measured only» rule — the §13.12 W9 precedent).
2. A platform without its own run is named «нет прогона», never «works».
3. The budgets are owner-set; the executor measures, the owner decides.
4. The verifier junction may re-use the machine but may not weaken it: no edge path skips the attest/e2e gates that the core path passes.

## 5. The roadmap W1… (the §5 ADR-0200 form; the slots fixate after the owner's verbatim)

| № | Scope | Depends on | Note |
|---|---|---|---|
| W1 | This wave (№1001): the ADR skeleton + the edge inventory (docs/research/edge-inventory-2026-11-24.md) + the verifier junction (§3) | an accepted ADR-0200 line closure | Executed by this naryad; zero product code |
| W2 | The CI-runner runs of the inventory numbers: size/startup/RSS measured on the runner (the release binary built by CI), an E-form artifact per release run + the workload-level startup definition (§3.4) | the owner's verbatim 24.11 (Wave 33) | Execution — naryad №1004 (gh#2067, волна В106 — W2); no budgets in W2 |
| W3 | The CI gates on size/startup/RSS + the recurring bench (the attest verify lane becomes a re-run lane) | the measured W2 numbers | the budgets are fixed by the owner's verbatim AFTER the W2 numbers — no budgets invented before (numbers measured only) |
| W4 | The cross-platform builds darwin/windows/arm64 — optional | the owner's call | The first to cut in the cut-order below |

Cut order if the line trims: the cross-platform builds first → the recurring bench → the size/startup gates. **Never cut:** the artifact provenance chain (SHA256SUMS/BUILD-INFO) and the e2e/attest coverage of the shipped binary.

## 6. Honest limits

The inventory is one platform, one artifact, one measurement session; the sandbox numbers do not transfer to the CI runner or to any other platform. Nothing in this ADR promises a smaller, faster, or more portable binary — it promises that the numbers that exist are measured, linked, and impossible to lose silently.
