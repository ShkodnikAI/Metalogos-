# ADR-0083: Unary Minus Fix

## Status
Accepted

## Context

Unary minus (`-42.0`) was silently producing `0.0` instead of `-42.0` when used in
entity declarations, let bindings, and function arguments. Users had to use the
workaround `0.0 - 42.0` which is error-prone and unreadable.

## Root Cause

Two interacting bugs in the parser:

1. **Atomic rule returned empty children**: `unary_minus = @{ MINUS ~ unary_expr }`
   used the `@` (atomic) modifier. In Pest, atomic rules do not expose their
   inner structure through `pair.into_inner()`, returning an empty iterator. The
   parser code `children[0]` accessed the first child (which didn't exist), fell
   through to `Expr::FloatLit(0.0)`, causing `-42.0` to desugar to `0.0 - 0.0 = 0.0`.

2. **Wrong child index**: Even with non-empty children, the code used `children[0]`
   which is the MINUS token itself, not the operand expression. Parsing MINUS as an
   expression produced `Expr::StringLit("-")`, which converted to `0.0` via
   `as_float()` soft-failure.

## Decision

### Fix 1: Remove atomic modifier
Changed grammar rule from:
```pest
unary_minus = @{ MINUS ~ unary_expr }
```
to:
```pest
unary_minus = { MINUS ~ unary_expr }
```

This allows `into_inner()` to return `[MINUS_pair, unary_expr_pair]`.

### Fix 2: Correct child index and fallback
Updated parser to use `children[1]` (the operand) with a fallback for atomic
rules that re-parses the inner content:
```rust
let inner_expr = if children.len() > 1 {
    parse_expression(children[1].clone())  // Non-atomic: skip MINUS, take operand
} else if !children.is_empty() {
    // Fallback for atomic rule: re-parse string after "-"
    // ...
}
```

### Desugaring approach
Unary minus is desugared to `BinaryOp(FloatLit(0.0), Sub, operand)` rather than
adding a dedicated `UnaryMinus` AST node. This minimizes interpreter changes
and leverages the existing binary operator evaluation.

## Consequences

- Unary minus `-42.0` now works correctly in all contexts (entities, calls, let)
- Nested unary minus works: `--x` desugars to `0.0 - (0.0 - x) = x` (double negation)
- The grammar change from atomic to non-atomic `unary_minus` introduces a
  theoretical ambiguity with `->` (ARROW) since both start with `-`. In practice,
  the parser resolves this correctly because ARROW is matched explicitly in
  declaration rules, not through the expression parser.
- Contract test: `examples/p5_unary_minus.mlog` (entity `-42.0` + pattern → `-84`)
