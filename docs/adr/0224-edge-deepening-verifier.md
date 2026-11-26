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
- The releases carry **linux-x86_64 only**; the CI-level per-platform runs landed (№1006, run 45549890075): **linux-arm64** (35 377 504 B, startup 0.003 s, workload 0.006 s verified, RSS 19.5 MiB — repeatable across runs) and **windows-x86_64** (31 899 648 B, startup 0.011 s, RSS n/a honestly, workload pin-mismatch — the §3.4 tripwire held, no numbers recorded) recorded with the run links in the inventory (§3 item 1); **darwin — нет прогона** (the macOS fleet allocation), no numbers invented.

## 2. Goals and non-goals

Goals: (a) the edge profile — the binary size, the startup, the memory, the constrained-platform readiness — becomes a **measured surface**: every number a run link, every gap named; (b) the verifier depth over the edge binary: which W1–W9 gates already cover it (§3), which are absent — named, not silent; (c) the honesty words: **an edge claim exists only with a run on the named platform** — no platform is claimed ready without its own run.

Non-goals (this wave): no product-code edge optimizations (the code gate is the owner's verbatim on this ADR); no new platform builds; no target/budget numbers — the budgets, if any, are fixed by the owner in §5 after the verbatim, not invented here.

## 3. The verifier junction (what covers the edge binary today — and what does not)

Already covers (measured, linked):

- The release provenance: SHA256SUMS + BUILD-INFO + SBOM per release — the artifact-level attestation chain (verified by the download of v0.40.1; the script is the inventory's companion tool).
- The e2e/attest/bench machine (№998) runs **against the compiled binary** — 10/10 RED-able; `attest verify` median 6.7 ms (the w9 bench, 23.11, the recorded run link in the artifact).

Absent (named, not silent):

1. No CI gate on size / startup / RSS — the W2 lane now produces the numbers per run (the first: run 45444498682, 24.11: 0.003 s / 75.3 MiB / 37 318 352 B, ubuntu26-azure; the sandbox 0.003 s / 25.0 MiB remains a separate recorded run), but nothing fails when they regress yet — RESOLVED 25.11 (№1008): the owner's verbatim delegation recorded in §5 W3; the gates are live in every E-artifact (PASS/FAIL per run, N/A where no number) with the values 2-3x the worst measured value.
2. No recurring bench gate — the 6.7 ms verify median is a single 23.11 record, not a re-run lane — RESOLVED 26.11 (№1010, gh#2085, PR #2098): the re-run lane landed — `.github/workflows/recurring-bench-attest-verify.yml` (weekly schedule + workflow_dispatch, non-blocking; a FAIL exits 2 for visibility), the w9 scenario per run (`mlog run line_demo.mlog` — the deterministic train — then `mlog attest verify line_demo.mlog --json` × N with `perf_counter`, the byte-for-byte pins enforced, the spawn effect verified every spawn; `scripts/recurring_bench_attest_verify.py` + the self-test). The median rides every run's E-artifact with the run link and the honest platform label, compared against the recorded baseline 6.7 ms — never silently re-baselined: a baseline rewrite only via a new run artifact + a §3 record here. The lane budget 20 ms is agent-proposed (the 2-3× rule ≈ 3× the worst measured single record; the №1008 delegation verbatim rides every E-artifact); the №1008 W3 gates themselves are not touched.
3. No cross-platform builds or verification — one artifact family, one platform.
4. No workload-level startup measurement — RESOLVED: the §3.4 machine landed (№1005: the pinned dag_demo program, the pin gate, the effect verified per run); the first runner run 45467303310 recorded 24.11 (median 0.004 s over 5 spawns, the effect verified each run, kernel 7.0.0-1016-aws); the number rides every edge-profile run from now.

### 3.4 The workload-level startup (the W2 definition — what counts as a program run)

The CLI floor (0.003 s, `--help`) measures the arg-parse path only. The **workload-level startup** is defined as: the wall-clock from the process spawn to the first observable effect of an **interpreted program run** — the completion of a pinned non-trivial evaluation that exercises the VM (not the CLI surface) — measured by the same methodology (the median of N spawns, `perf_counter`) on the named platform, with the workload program pinned in the artifact (byte-identical input, the number reproducible). The measurement landed in a W2+ step (№1005, 24.11): the first recorded run https://github.com/ShkodnikAI/Metalogos-/actions/runs/45467303310 (the merge-commit push of 8bebdd81) — the pinned program `examples/dag_demo.mlog` (the sha256 pin ae89af54…3da599, byte-identical, enforced by the pin gate and the self-test tripwire), the expected effect `examples/dag_demo.expected` (pin 9337ffb9…d9cc2624), **median 0.004 s over 5 spawns, the effect verified each run**, on the CI runner (kernel 7.0.0-1016-aws; the fleet varies run to run — the honest label carries it, no average invented). The number rides the E-artifact per run from now (numbers measured only; the W3 budgets are live from №1008 — the verdicts ride every E-artifact, see §5 W3).

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
| W2 | The CI-runner runs of the inventory numbers: size/startup/RSS measured on the runner (the release binary built by CI), an E-form artifact per release run + the workload-level startup definition (§3.4) | the owner's verbatim 24.11 (Wave 33) | Executed — №1004 (gh#2067, волна В106 — W2): the machine (PR #2069) + the first runner run 45444498682 (0.003 s / 75.3 MiB / 37 318 352 B; ubuntu26-azure); no budgets in W2 |
| W3 | The CI gates on size/startup/RSS + the recurring bench (the attest verify lane becomes a re-run lane) | the measured W2 numbers | RESOLVED 25.11 (№1008): the owner's verbatim (the delegation of 25.11 — the full quote carried verbatim in every E-artifact and in gh#2080) accepted the agent-proposed gates: workload median ≤ 30 ms (re-baselined №1009, 25.11: the first windows workload measurement is 16 ms — the linux-only 10 ms would false-FAIL on a healthy windows; the declared 2-3×-of-worst ratio kept), CLI-floor ≤ 25 ms, RSS peak ≤ 150 MiB, size ≤ 45 000 000 B (each 2-3× the worst W2/W4 measurement; the size gate deliberately ~1.2× — the earliest bloat signal). The verdicts ride every E-artifact; a FAIL exits 2 (the lanes are non-blocking — visibility without a release block). The recurring-bench part (the attest verify lane → a re-run lane) stays a separate follow-up — RESOLVED 26.11 (№1010, gh#2085, PR #2098): the lane `recurring-bench-attest-verify` landed (the weekly schedule + the dispatch, non-blocking, the E-form artifact per run; the budget 20 ms agent-proposed ≈ 3× the 6.7 ms w9 record, the delegation verbatim rides the artifact; the baseline not re-baselined by the lane — see §3 absent-2). |
| W4 | The cross-platform builds darwin/windows/arm64 — optional | the owner's call | The first to cut in the cut-order below |

Cut order if the line trims: the cross-platform builds first → the recurring bench → the size/startup gates. **Never cut:** the artifact provenance chain (SHA256SUMS/BUILD-INFO) and the e2e/attest coverage of the shipped binary.

## 6. Honest limits

The inventory is one platform, one artifact, one measurement session; the sandbox numbers do not transfer to the CI runner or to any other platform. Nothing in this ADR promises a smaller, faster, or more portable binary — it promises that the numbers that exist are measured, linked, and impossible to lose silently.
