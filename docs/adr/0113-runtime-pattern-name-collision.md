# ADR-0113: Pattern-name collision warnings on `run` / `serve`

**Status:** Accepted
**Date:** 2026-09-03
**Naryad:** #163

## Context

`mlog check` already rejects duplicate pattern names (`src/semantic.rs`,
`format_duplicate_pattern`). `mlog run` and `mlog serve` load modules
through the tree-walking interpreter (`src/interpreter/modules.rs`) which
registered patterns with `HashMap::insert` and silently kept the last
definition.

That split is why a production `app.mlog` can show dozens of
`duplicate pattern` findings under `check --root` while `serve` starts
without a word.

## Decision

### 1. Default is Warning, not Error

Existing deployments that already run with silent overwrites must keep
starting. A collision prints `warning: duplicate pattern: NAME (...origins...)`
to stderr and the load continues (last definition still wins, same as
before).

### 2. Opt-in Error via `--strict` / `METALOGOS_STRICT=1`

`mlog run --strict` and `mlog serve --strict` set `METALOGOS_STRICT=1`.
`Interpreter::new()` reads that variable. The same flag is also available
as `Interpreter::set_strict_pattern_names(true)` for tests and embedders.
In strict mode the load returns `Err` and the process exits before
serving traffic.

### 3. One diagnostic helper, two call sites

Wording lives in `semantic::format_duplicate_pattern` so `check` and the
runtime loader stay aligned. The loader adds origin paths
(`dept/utils` vs `<program>`) when it has them.

Registration is funneled through `Interpreter::register_pattern` /
`register_learnable` — used from both `modules.rs` (imports) and
`execution.rs` (entry file). No second copy of the HashSet logic.

### 4. Same-origin re-insert is not a collision

The loader historically re-executes a module that is imported twice
(there is circular-import detection, not a loaded-set). Replaying
`pattern Foo` from the same module path is not a name clash and must
not warn. A clash is two *different* origins claiming the same name.

## Consequences

- `mlog serve` of a colliding tree now prints the collisions at startup
  and still binds the port.
- Operators who want a hard gate flip `--strict` or the env var.
- `mlog check` behaviour is unchanged (still Error).
