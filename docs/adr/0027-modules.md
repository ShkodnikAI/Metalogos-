# ADR-0027: Module System with Namespaces

**Status:** Accepted
**Date:** 2026-06-02
**Phase:** 5.4 — Language Completeness

## Context

Before Phase 5.4, all patterns, entities, and builtins existed in a single flat namespace. This causes problems as the language grows: standard library patterns like `trim`, `abs`, `split` pollute the global namespace and risk colliding with user-defined names. Additionally, without a module system, code cannot be organized into reusable, namespaced units.

The skill document (5.4) proposes `import path as alias` syntax with qualified calls `alias.PatternName(...)`, relative imports `./path`, standard library imports `std/path`, and circular import detection.

## Decision

### Import Syntax

```mlog
import std/string as str      // namespaced: str.trim(s), str.replace(s, old, new)
import std/math as m         // namespaced: m.abs(-42.0), m.min(a, b)
import ./my_utils             // global merge: all patterns available without prefix
```

### Module Resolution

1. `import path as alias` — loads `path.mlog`, registers alias→path mapping, merges patterns into global `self.patterns`.
2. `import path` (without `as`) — same behavior, alias = path (e.g., `./my_utils`).
3. `std/string` → `base_dir/std/string.mlog` (standard library).
4. `./my_utils` → `base_dir/./my_utils.mlog` (relative to importing file).
5. `pkg/path` → reserved for future dependency system (currently resolves same as std).

### Qualified Calls

`alias.PatternName(args)` is parsed as `Expr::QualifiedCall { module, function, args }`. Resolution:
1. Verify `module` exists in `module_namespaces` (import was declared).
2. Look up `function` in `self.patterns` (modules merge patterns globally).
3. If not found, return error: `undefined pattern 'fn' in module 'mod'`.

This design means modules share a single global pattern namespace — qualified calls only gate access through a namespace check. This avoids the complexity of per-module pattern isolation while still preventing accidental name collisions.

### Circular Import Detection

Uses a `loading_stack: Vec<String>` on the interpreter. Before loading a module, check if its path is already in the stack. If so → error: `circular import detected: a -> b -> a`.

### Stdlib Design

The `std/` directory contains `.mlog` files that wrap double-underscore builtins (`__trim`, `__abs`, etc.) into idiomatic Metalogos patterns. This layer of indirection means:

1. Standard library code is written in Metalogos itself (self-hosting enabler).
2. Builtins with `__` prefix are implementation details, not public API.
3. Users import `std/string` and call `str.trim()`, never `__trim()`.

Builtins added for stdlib support:
- `__trim(s)`, `__replace(s, old, new)`, `__split(s, sep)`, `__join(items, sep)`
- `__abs(n)`, `__min(a, b)`, `__max(a, b)`, `__clamp(val, lo, hi)`, `__round(n)`
- `__first(items)`, `__last(items)`

### Grammar Changes

- `import_decl` rule: `IMPORT_KW ~ import_path ~ (AS_KW ~ IDENT)?`
- `import_path` rule: `("./" ~ segments) | segments` (separate rule to avoid atomic parser issues)
- `qualified_call_expr` rule: `IDENT ~ "." ~ IDENT ~ "(" ~ args ")"` (placed before `call_expr` and `field_expr` in `unary_expr`)
- `IDENT` updated to allow leading underscore `(ASCII_ALPHA | "_")` for `__trim` etc.
- `unary_minus` rule added for negative literals (`-42.0`)

### Unary Minus Note

Unary minus (`-42.0`) was added during Phase 5.4 but has a known limitation: it doesn't work in all contexts due to pest parser precedence. The workaround `0.0 - 42.0` is fully functional. This will be fixed in a follow-up ADR.

## Prior Art

- **Rust:** `mod`, `use`, `crate::`, `self::`, relative paths, no circular imports.
- **Elixir:** `import Module, only: [funcs]`, `alias Module, as: NewName`, compile-time circular detection.
- **Python:** `import module`, `from module import name`, `as` aliasing, runtime circular detection.
- **Lua:** `require("module")`, returns table, no namespace enforcement.
- **JavaScript:** `import { name } from 'module'`, `import * as alias`, static analysis for circular deps.

## Consequences

- **Positive:** Standard library patterns no longer pollute global namespace.
- **Positive:** Qualified calls `str.trim()` read naturally and self-document module origin.
- **Positive:** Circular import detection prevents infinite loops at module load time.
- **Positive:** `import path` (without `as`) preserves backward compatibility.
- **Neutral:** Module isolation is shallow — patterns merge globally, qualified calls only check the namespace exists, not that the pattern originated from that module. This is acceptable for Phase 5.4 scope; true per-module isolation can be added later if needed.
- **Negative:** Unary minus `-42.0` has edge cases. Workaround: `0.0 - 42.0`.
