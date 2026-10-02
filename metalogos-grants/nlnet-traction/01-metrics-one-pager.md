# Metalogos — the honesty metrics (one pager)

Draft for the NLnet/Restack traction package (№549). Every number below
is a repository fact with its date; machine-generated where a generator
exists. No projections.

## The snapshot (2026-10-03, main `6e66d3d`)

| Fact | Value | Source / date |
|---|---|---|
| Language version | 0.27.1 (published 2026-09-29, tag `v0.27.1`) | releases / 2026-09-29 |
| Workspace crates | the language root + `mlogpkg` + `mlog-lsp` + `metalogos-reflex` — the generative contour is its own crate (the №545 split) | `Cargo.toml` members / 2026-10-02 |
| Built-in surface | 507 builtins across 45 modules | `scripts/gen_metrics.py` |
| Typed signatures | 162/507 = **31.95%** (floor 3195 bp, «only up» since №467) | `type_signature_share.py` / 2026-10-03 |
| TW/VM duplicate names | 0 (threshold 0) | `count_duplicated_names.py` / 2026-10-03 |
| vm.rs mirror marks | 0 | `mirror_counter.py` / 2026-10-03 |
| Tech-debt counters | ignore 52 · ignore_todo 16 · dead_code 33 (floor 37) · example_uncovered 0 — under their floors, «only down» | `debt_counters.py --gate` / 2026-10-03 |
| Blocking checks | 27 machine-read cells over run/check/compile/serve (`blocking_checks.tsv`, №535), each bound to its proving test | `blocking_checks_sync.py` / 2026-10-03 |
| CI checks per commit | 45 (44 success + 1 skipped; blocking + non-blocking lanes, TW and VM parity) | check-runs API / 2026-10-03 |
| Test files | 281 integration test files (+ the unit lanes) | `ls tests/*.rs` / 2026-10-03 |
| Example programs | 245 .mlog programs (each fenced snippet in docs runs against the live grammar) | `ls examples/*.mlog` |
| ADRs | 173 accepted architecture decisions | `docs/adr/` |
| Changelog | ~614 KB — every wave documented, single file across crates | `CHANGELOG.md` |
| Development pace | 846 commits since 2026-09-01 (one month) | `git log` / 2026-10-03 |
| Release gate 0.28 | **SATISFIED** (the absolute goals: typed share ≥ goal, 0 open High in the server path, the transfer quorum ≤ 1/3, the serve-e2e inventory done) | `unfreeze_gate.py --gate-target 0.28 --office-tests pass` / 2026-10-03 |

## The honesty discipline behind the numbers

- **The floors move only up, the debts only down** — a metric that
  regresses fails CI by construction (the №467/№462/№468 gates); the
  baselines are checked-in files with a written history.
- **Fail-closed everywhere**: a missing record is a failure; a fact
  without a machine source is itself a failure (№525).
- **The honest-Unknown rule**: of the 507 builtins, 162 carry a typed
  signature and the remaining 345 carry the explicit `Unknown` — never
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
