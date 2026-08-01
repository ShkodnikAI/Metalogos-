# ADR-0069: slice() builtin for lists

## Status
Accepted

## Context

FOSVED Office production code (`app.mlog:1293`) calls `slice(parts, 2.0, len(parts))` to extract a sub-list before joining. Metalogos 0.12.0 has no `slice` builtin, and no alternative exists:

- `sublist`, `sub_list`, `take`, `drop`, `skip`, `splice` — none present
- Index syntax `list[a..b]` — not implemented (IndexAccess only accepts Float)

This is a language gap, not an application bug. The fix belongs in the language runtime.

## Decision

Add `slice(list, start, end) -> List` as a builtin, mirroring `substring` semantics exactly:

- Semi-open interval `[start, end)`
- `start >= len` returns empty list (soft-failure)
- `end > len` clamps to `len`
- `start >= end` returns empty list
- Non-list first argument returns error

Registered in `BUILTIN_REGISTRY` (arity 3, category "list") and `funcs.insert`.

## Alternatives

(a) **Implement `list[a..b]` syntax** — touches PEG grammar, interpreter, compiler, and VM backends. Correct long-term solution but disproportionate for a single missing operation.

(b) **Rewrite FOSVED to use `each` + index check** — treats the symptom, leaves the language gap for all future users.

(c) **Builtin `slice`** — chosen. Minimal change (one function, two registration lines), mirrors existing `substring` contract, unblocks FOSVED 0.12.0 upgrade immediately.

## Prior art

- Python: `list[start:end]`
- Rust: `&v[start..end]`
- JavaScript: `Array.prototype.slice(start, end)`
- Metalogos `substring(s, start, end)`: identical soft-failure semantics for strings
