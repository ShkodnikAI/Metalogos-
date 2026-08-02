# ADR-0080: Module size policy

**Status:** accepted
**Date:** 2026-08-02
**Context:** Наряд №38 Block 1

## Problem

Наряд №37 introduced an 800-line hard limit on all files in `src/`.
The rule was effective at breaking up monoliths:

| File | Before | After |
|------|--------|-------|
| `builtins.rs` | 10,838 | 588 (mod.rs) |
| `parser.rs` | 4,921 | 128 (mod.rs) |
| `interpreter.rs` | 5,073 | 539 (mod.rs) |

However, the 800-line limit proved too aggressive for files with a clear,
coherent single topic. Two production files exceeded it after the split:

| File | Lines | Topic |
|------|------:|-------|
| `builtins/server.rs` | 2,579 | Server/message/db/human builtin handlers |
| `interpreter/mod.rs` | 2,178 | Interpreter struct, run(), eval dispatch |

Neither file is a "monster" — both have a clear single responsibility.
Further splitting would slice along artificial boundaries within a
coherent module, harming readability.

## Decision

### New rule

```
Production module:  not more than ~2,000 lines
Test module:       no limit
```

Rationale:
- 2,000 lines is a comfortable size for a single Rust source file with a
  clear topic and consistent structure.
- The original 800-line limit was an emergency tool against 10,000-line
  monoliths. That emergency is over.
- Test modules are excluded because they benefit from grouping related
  test cases in one place, and test density varies naturally.

### Actions taken in this Наряд

1. **`interpreter/mod.rs`**: extracted `execution.rs` (1,645 lines) containing
   `run()`, `eval_expr()`, `eval_statements()`, `eval_binop()`, and related
   dispatch functions. Result: 2,178 → 539 lines.

2. **`builtins/server.rs`**: extracted `office.rs` (1,724 lines) containing
   human interaction, goals, todos, recipes, DAG, semantic search, and config
   builtins. Result: 2,579 → 865 lines.

### Current largest production files

| File | Lines |
|------|------:|
| `vm.rs` | 1,934 |
| `server.rs` | 1,744 |
| `builtins/office.rs` | 1,724 |
| `builtins/tests.rs` | 1,335 (test, exempt) |
| `parser/tests.rs` | 2,064 (test, exempt) |

No production file exceeds 2,000 lines.

## Builtin form preserved during split (Block 3 audit)

At Наряд №38 Block 3 acceptance, a concern was raised that the split modules
(`collections.rs`, `http.rs`, `json.rs`, `crypto.rs`, `math.rs`, `io.rs`,
`llm.rs`, `core.rs`) might contain zero `fn` declarations — i.e. builtins
might have been re-registered as closures instead of named functions.

**Audit result: form was preserved.** Checked against the original
monolithic `builtins.rs` (commit `d3ab3b2`, the earliest version with
named `fn builtin_*` declarations):

- **Before split:** All builtins were `fn builtin_xxx(args: &[Value]) -> Result<Value, String>`
  registered via `funcs.insert("xxx", builtin_xxx as BuiltinFn)`.
- **After split:** Same form. Every extracted module (`collections.rs`: 12 fn,
  `http.rs`: 24 fn, `math.rs`: 13 fn, `json.rs`: 8 fn, `crypto.rs`: 8 fn,
  `io.rs`: 10 fn, `llm.rs`: 5 fn, `core.rs`: 3 helper fn) uses identical
  `pub(crate) fn builtin_xxx` declarations.

No closure-based re-registration occurred. The split was a pure code move.

## Consequences

- The 800-line rule in `docs/refactoring-split-plan.md` is superseded by
  this ADR.
- Future splits should target ~2,000 lines per production file, not 800.
- Test files (`*_tests.rs`, `tests/`) have no size constraint.
- Builtin function form (named `fn`) was preserved during №37 split —
  confirmed by audit.
