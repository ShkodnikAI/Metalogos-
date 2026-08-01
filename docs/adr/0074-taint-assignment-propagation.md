# ADR-0074: Taint tracking — assignment propagation

**Date:** 2026-08-01
**Status:** accepted
**Context:** Наряд №33 Block 5

## Problem

The audit module's taint tracker only marked variables as tainted when
the RHS was a direct function call to a known source (`call_llm`, `env`,
`json_body`, etc.). Taint did **not** propagate through variable assignments:

```metalogos
let x = call_llm("prompt")   // x marked LlmOutput ✓
let y = x                     // y NOT marked ✗
let _ = respond(y)            // should be flagged, but isn't ✗
```

This meant any indirection through an intermediate variable defeated
all three taint-based audits: HTML_INJECTION, SECRET_LEAK, OPEN_REDIRECT.

## Decision

Added `get_expr_taint(expr, tracker)` helper that recursively extracts
taint from expressions, and `binding_taint(value, tracker)` that checks
direct source functions first then falls back to expression-level propagation.

### Propagation rules

| Expression | Taint result |
|-----------|---------------|
| `Expr::Ident(var)` | var's taint |
| `Expr::FnCall("render"/"escape_html", _)` | Sanitized (overrides) |
| `Expr::FnCall(_, args)` | first non-Sanitized arg taint |
| `Expr::BinaryOp(left, _, right)` | either side's taint |
| `Expr::FieldAccess(obj, _)` | obj's taint |
| `Expr::IndexAccess(_, idx)` | idx's taint |
| `Expr::IfElse(_, then, else)` | then or else taint |
| Literals | None (clean) |
| All other variants | None (conservative) |

### Where propagation applies

All three `analyze_scope` functions updated:
1. **HTML_INJECTION** (`check_html_injection`) — LlmOutput → respond()
2. **SECRET_LEAK** (`check_secret_leak`) — Secret → http_post()
3. **OPEN_REDIRECT** (`check_open_redirect`) — UserInput → redirect()

Both `LetBinding` and `Assign` statements use `binding_taint()`.

## What is NOT covered (intentionally)

- **Struct field access**: `x.field` propagates from `x`, but `x = { field: tainted }`
  does not mark the struct's field as tainted independently. This requires
  per-field taint which is significantly more complex.
- **List element taint**: `[clean, tainted]` — the whole list should be tainted,
  but currently only direct variable references are tracked.
- **Cross-file analysis**: `import` is not tracked. Taint from one module's
  variable does not propagate to another module's usage.
- **Pest spans**: Positions are approximate (substring search in source text),
  not precise Pest spans. This is a separate improvement.
- **Multi-taint**: A variable can only have one taint kind. If `x` is tainted
  LlmOutput and then `y = x + secret_var`, `y` gets whichever taint is found
  first. Full multi-taint requires a `HashSet<TaintKind>` per variable.

## Consequences

- Audit now catches indirect taint flows through assignments.
- All existing tests continue to pass (377/377).
- Adding new Expr variants to AST requires updating `get_expr_taint`.
