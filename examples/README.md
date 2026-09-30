# The examples/ catalog — the living corpus

The top level of `examples/` holds the LIVE corpus: every `.mlog` program
here is parsed by the CURRENT grammar and is verified by at least one of:

- a golden sidecar (`.expected` / `.error` — `tests/golden.rs` runs it);
- a named mention in `tests/`, `benches/`, `scripts/` or `.github/`
  (a CI check that exercises it);
- the crosscheck/bench suites.

`tests/naryad_513_example_coverage.rs` enforces the coverage (the audit
28.09 C-09 hole must not reopen), and `scripts/ci/debt_counters.py`
counts the uncovered examples (`example_uncovered`, movement only down).

## examples/compat/ — the honest archive (№533)

`examples/compat/` holds the programs the CURRENT grammar DOES NOT parse:
historical syntax kept for its precedent value, never deleted (removal
would erase the syntax history; rewriting them to the new grammar is a
separate decision, sanctioned by its own naryad — not done silently).

Each archived file carries the honesty header in its first lines:

```
// COMPAT-513 (archived to examples/compat/ by №533): historical syntax —
// not parsed since v0.27.x. See rule №513 (gh#797).
```

The rules:

1. The grammar tests and the example-coverage test NEVER enter
   `examples/compat/` — the exclusion is by PATH, not by a tag inside the
   file (the №513 tag-scan is retired; a tagged file in the LIVE catalog
   is now debt, not a pass).
2. Nothing in `compat/` is a test fixture — no `.expected`/`.error`
   sidecars are read from there.
3. To resurrect an archived example: rewrite it to the current grammar,
   give it a golden sidecar or a named check, and move it back to the
   top level — in its own naryad.
