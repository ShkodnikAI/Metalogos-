# Metalogos — the honesty metrics (one pager)

Draft for the NLnet/Restack traction package (№549; refreshed by №577).
Every number below is a repository fact with its date; machine-generated
where a generator exists. No projections.

## The snapshot (2026-10-05, main `7b10f84` — the pre-M1 refresh, №577)

| Fact | Value | Source / date |
|---|---|---|
| Language version | 0.28.0 (published 2026-10-03, tag `v0.28.0`) — the first release with supply-chain assets; 0.28.1 (the security follow-up, the wave-29 audit fixes) is staged | releases / 2026-10-03; gh#1004 |
| Built-in surface | 509 builtins across 45 modules | `scripts/gen_metrics.py` / 2026-10-05 |
| Typed signatures | **186/509 = 36.54%** (floor 3654 bp, «only up» since №467) — the 0.29 goal (3500 bp) already exceeded | `type_signature_share.py` / 2026-10-05 |
| Precise signatures | **104/509 = 20.43%** (floor 2043 bp, the second share, №560) | `type_signature_share.py` / 2026-10-05 |
| TW/VM duplicate names | 0 (threshold 0) | `count_duplicated_names.py` / 2026-10-05 |
| Mirror marks (whole src/) | 6 (threshold 6) | `mirror_counter.py` / 2026-10-05 |
| Tech-debt counters | ignore 17 · ignore_todo 0 · dead_code 33 · example_uncovered 0 — «only down» | `debt_counters.py` / 2026-10-05 |
| Test coverage | **76.60% line** (the floor 76% advisory, the owner decision 2А — flips blocking after two stable waves) | `coverage_baseline.txt` (measured @ e662ffd) / 2026-10-04 |
| Blocking checks | 31 machine-read cells over run/check/compile/serve (`blocking_checks.tsv`, №535), each bound to its proving test | `blocking_checks_sync.py --count` / 2026-10-05 |
| Test files | 251 integration test files in the root crate (+ the server-crate and unit lanes) | `ls tests/*.rs` / 2026-10-05 |
| Example programs | 245 .mlog programs (each fenced snippet in docs runs against the live grammar) | `ls examples/*.mlog` / 2026-10-05 |
| ADRs | 174 accepted architecture decisions | `docs/adr/` / 2026-10-05 |
| Development pace | 882 commits since 2026-09-01 (five weeks) | `git log` / 2026-10-05 |
| Release gate | the 0.28 goals **SATISFIED and PUBLISHED**; the 0.29 absolute goals owner-fixed 2026-10-04 (ADR-0181) — the typed-share goal already met | `unfreeze_gate.py` / 2026-10-05 |

## The previous snapshot (2026-10-03, release commit `3576887` — tag `v0.28.0`)

| Fact | Value | Source / date |
|---|---|---|
| Language version | 0.28.0 (published 2026-10-03, tag `v0.28.0`) — the first release with supply-chain assets: binary, CycloneDX SBOM, BUILD-INFO, SHA256SUMS, two Sigstore attestations | releases / 2026-10-03 |
| Workspace crates | the language root + `mlogpkg` + `mlog-lsp` + `metalogos-reflex` — the generative contour is its own crate (the №545 split) | `Cargo.toml` members / 2026-10-02 |
| Built-in surface | 509 builtins across 45 modules | `scripts/gen_metrics.py` |
| Typed signatures | 165/509 = **32.41%** (floor 3241 bp, «only up» since №467) | `type_signature_share.py` / 2026-10-03 |
| Precise signatures | 84/509 = 16.50% (floor 1637 bp, the second share, №560) | `type_signature_share.py` / 2026-10-03 |
| TW/VM duplicate names | 0 (threshold 0) | `count_duplicated_names.py` / 2026-10-03 |
| Mirror marks (whole src/) | 7 (threshold 7, the №564 whole-tree metric) | `mirror_counter.py` / 2026-10-03 |
| Tech-debt counters | ignore 52 · ignore_todo 16 · dead_code 33 (floor 37) · example_uncovered 0 — under their floors, «only down» | `debt_counters.py --gate` / 2026-10-03 |
| Blocking checks | 27 machine-read cells over run/check/compile/serve (`blocking_checks.tsv`, №535), each bound to its proving test | `blocking_checks_sync.py` / 2026-10-03 |
| CI checks per commit | 46 (blocking + non-blocking lanes, TW and VM parity; MSRV contract job added №555) | check-runs API / 2026-10-03 |
| Test files | 293 integration test files (+ the unit lanes) | `ls tests/*.rs` / 2026-10-03 |
| Example programs | 245 .mlog programs (each fenced snippet in docs runs against the live grammar) | `ls examples/*.mlog` |
| ADRs | 173 accepted architecture decisions | `docs/adr/` |
| Changelog | ~629 KB — every wave documented, single file across crates | `CHANGELOG.md` |
| Development pace | 866 commits since 2026-09-01 (one month) | `git log` / 2026-10-03 |
| Release gate 0.28 | **SATISFIED and PUBLISHED** (the absolute goals: typed share ≥ goal, 0 open High in the server path, the transfer quorum ≤ 1/3, the serve-e2e inventory done) — the release shipped 2026-10-03 | `unfreeze_gate.py --gate-target 0.28 --office-tests pass` / 2026-10-03 |

## The honesty discipline behind the numbers

- **The floors move only up, the debts only down** — a metric that
  regresses fails CI by construction (the №467/№462/№468 gates); the
  baselines are checked-in files with a written history.
- **Fail-closed everywhere**: a missing record is a failure; a fact
  without a machine source is itself a failure (№525).
- **The honest-Unknown rule**: of the 509 builtins, 165 carry a typed
  signature and the remaining 344 carry the explicit `Unknown` — never
  a fictitious precise type; the audit trail of every typing step is
  in the registry comments and the baseline history.
- **Two-backend parity**: the tree-walking interpreter and the bytecode
  VM are cross-checked (output parity gate) — a program's result does
  not depend on the backend that ran it.
- **The denials are explainable**: every refusal (compile-time taint,
  runtime sink gate, grant exhaustion) carries the class, the rule and
  the exact node — and the irreversible actions leave a signed,
  externally verifiable ledger trail (the demo below shows both).

## The demo (one command)

```bash
bash scripts/demo_traction.sh
```

Three legs over the public examples verbatim: the static taint refusal
(the private camera frame cannot reach a file sink — denied at compile
time with the exact node and rule), the live serve loop, and the
action-ledger chain (grant → allow → exhaust → deny → key rotation →
export → `mlog ledger verify` — the Ed25519 chain verifies WITHOUT the
Metalogos runtime). See `02-demo.md`.
