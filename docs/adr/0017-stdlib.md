# ADR 0017: Standard Library (stdlib) and Import Mechanism

**Status**: Accepted
**Date**: 2026-06-01
**Phase**: 3.2

## Context

METALOGOS needs a standard library to provide commonly-used operations (string manipulation, math, collections) without requiring users to define them in every program. The language previously had only hardcoded builtins (`upper`, `lower`, `len`, etc.) with no mechanism for users to import reusable modules.

The `mlog repl` and `mlog run` CLI commands (Phase 3.1) need to automatically load stdlib modules when programs use `import std/X` statements. The REPL should also support import within sessions.

## Decision

### 1. Import Syntax

Add `import <module_path>` as a top-level declaration:

```
import std/string
import std/math
import std/collections
```

Module paths use `/` as separator, resolving to `<std_root>/<module_path>.mlog`.

### 2. Resolution Strategy

- **std_root**: Default = current working directory. For `mlog run <file>`, resolves relative to the source file's parent directory. For REPL, uses CWD.
- **Resolution**: `import std/string` -> `<std_root>/std/string.mlog`
- **Recursive**: Imported files can contain their own `import` statements (with cycle detection via `imported_modules: HashSet<String>`)
- **Inline expansion**: Imports are resolved during `Interpreter::run()` preprocessing, before declaration execution. The imported declarations are inlined into the main program's declaration list.

### 3. std/ Module Structure

Three modules, each as a `.mlog` file containing pattern declarations:

- **std/string.mlog**: `trim(s)`, `replace(s, old, new)`, `split(s, sep)`, `join(items, sep)`
- **std/math.mlog**: `abs(n)`, `min(a, b)`, `max(a, b)`, `clamp(val, lo, hi)`, `round(n)`
- **std/collections.mlog**: `first(items)`, `last(items)`, `push(items, item)`, plus native `map`/`filter`/`reduce`

### 4. Implementation Architecture

**Two-layer pattern wrapper**:
- Rust builtins use `__` prefix (e.g., `__trim`, `__replace`) to avoid name collisions
- `.mlog` patterns provide the public API (e.g., `pattern trim(s: String) -> String { return __trim(s) }`)
- When `trim("  hello  ")` is called: pattern lookup -> body evaluates `__trim(s)` -> builtin execution

**Collections (map/filter/reduce)**:
- These require interpreter context (access to pattern registry)
- Implemented as native methods on `Interpreter`, guarded by `collections_loaded` flag
- The flag is set when `import std/collections` is processed
- Signature: `map(list, "pattern_name")`, `filter(list, "pattern_name")`, `reduce(list, "pattern_name", init)`

### 5. Value::List

Added `Value::List(Vec<Value>)` to the runtime value enum, with:
- Display as `[item1, item2, ...]`
- `type_name()` returns `"List"`
- `as_float()` returns list length (enables `len` semantics)
- Used by split/join/push/map/filter/reduce

### 6. Grammar Changes

- Added `import_decl`, `module_path`, `IMPORT_KW`, `SLASH` rules
- Extended `IDENT` to allow leading underscore: `(ASCII_ALPHA | "_") ~ ...`
- Added `"import"` to `step_ident` negative lookahead

## Contract (Golden Test)

`examples/p3_stdlib.mlog`:
```
import std/string
entity raw: String = "  hello world  "
entity cleaned: String = trim(raw)
entity result: String = replace(cleaned, "world", "METALOGOS")
flow Main { input: String = result -> output }
```

Expected: `hello METALOGOS`

## Consequences

- **Positive**: Clean separation between Rust implementation details (`__` builtins) and user-facing API (stdlib patterns). Import mechanism enables modular code organization. Collections powered by named patterns align with METALOGOS' pattern-first philosophy.
- **Negative**: `map`/`filter`/`reduce` are not pure .mlog patterns (they're special-cased in the interpreter). No comment support in .mlog files (std files cannot use `//` comments).
- **Future**: Could add parametric list types (`List<T>`), comment support in grammar, and package manifests for versioned stdlib distribution.
