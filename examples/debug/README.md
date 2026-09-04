# Debug Examples

This subdirectory holds `.mlog` files that are NOT golden contracts.
They are typically bug-reproduction cases from past naryads, kept for
historical reference and manual replay.

## Why these are not in `examples/`

The main `examples/` directory is the golden corpus: every `.mlog`
file there is expected to have a corresponding `.expected` (or `.error`)
pair. Files without a pair are silently ignored by `tests/golden.rs::collect_pairs`,
which creates ambiguity — is the missing pair intentional or a forgotten file?

`examples/debug/` makes the intent explicit: these files are deliberately
single-source (no `.expected`), deliberately not golden contracts.

## Files

- `bug_route_pattern.mlog` — Наряд №8: bug reproduction for "route
  handler calling user-defined pattern". Fixed by commit `9d454da`
  (`fix(n8): route handlers invoke user-defined patterns`). Kept
  because the original bug reproduction is a useful reference for
  regression testing if the route handler code is touched again.

## How to run

```bash
cargo run --bin mlog -- check examples/debug/bug_route_pattern.mlog
cargo run --bin mlog -- run examples/debug/bug_route_pattern.mlog   # if it has a Main flow
```

## Adding new debug files

If you have a bug reproduction that doesn't fit the golden contract
shape (no deterministic stdout, requires env vars, requires live HTTP,
etc.), put it here. Add a `//`-comment at the top describing the bug
and the naryad that fixed it.

When (and if) the file becomes a stable contract, move it to
`examples/` and add the `.expected` / `.error` pair.
