# ADR-0098: Registry–Dispatcher Sync

**Status:** accepted

## Context

BUILTIN_REGISTRY (`spec!` macro) and the dispatcher (`funcs.insert`) in
`builtins/mod.rs` are two independently maintained lists of builtin
functions. `mlog check` uses the registry for arity validation and
unknown-function detection; the runtime uses the dispatcher for
call resolution.

When a function is added to the dispatcher but not the registry (or vice
versa), `mlog check` either reports false positives (rejecting valid code)
or misses real errors (accepting calls to non-existent functions).

This divergence has occurred four times (Наряды №33, №36, №50, №55)
because there is no automated check that keeps the two lists in sync.

## Decision

1. **Close all current holes.** 6 functions were missing from the
   registry: `db_execute` (arity was 1, should be 1..2 per ADR-0068),
   `to_int`, `cron_add`, `cron_list`, `cron_remove`, `cron_run`.

2. **Add an automated set-difference test.** `tests/registry_sync_check.rs`
   verifies that `builtin_count()`, `builtin_names()`, and
   `builtin_name_set()` agree with `BUILTIN_REGISTRY`.  A separate
   test (`naryad_55_fixes_present`) asserts the six specific fixes.

3. **Preserve the existing hardcoded arity test.** `registry_arity_check.rs`
   tests per-function arity values (min/max).  The new sync test catches
   missing entries; the existing test catches wrong arities.  Different
   failure modes, both needed.

4. **Registry-only categories.** Functions in categories `stub`,
   `stateful`, `graph`, `mtree`, `cron`, `test`, `std`, `convert`, `web`
   are intentionally in the registry without a dispatcher entry — they are
   handled by interceptors or specialized invoke methods.  This is
   documented in the test's allowlist.

## Consequences

- `mlog check` no longer reports false positives on `db_execute(…, params)`
  and `to_int()` in the FOSVED office code.
- Future `funcs.insert` additions without a matching `spec!` will be
  caught by CI (the `registry_names_are_consistent` test fails).
- `cargo fmt` alignment in `registry.rs` must match project style —
  single-line comments go after the closing parenthesis, arrays with
  many items use multi-line formatting.
