# ADR 0024: let bindings + if/else expressions

**Status:** Accepted
**Date:** 2026-06-01
**Phase:** 5.1 — Language Completeness

## Context

Phase 4 delivered a complete tree-walking interpreter with 7 pillars. Pattern bodies could only contain a single `return expression` — no local computation or branching. This made patterns trivial: bind parameters and return one expression.

Two features requested:
1. **`let` bindings** — `let x = expr` inside patterns
2. **`if/else` as expressions** — `if cond then expr1 else expr2` returning values

Contract: nested `else if` pattern with string concatenation.

## Decision

### `let` bindings

Grammar: `statement = { let_binding | return_stmt }` where `let_binding = { "let" ~ IDENT ~ "=" ~ expression }`.

AST: `Statement::LetBinding { name: String, value: Expr }` added alongside `Statement::Return(Expr)`.

Interpreter: `eval_statements()` takes `&mut HashMap<String, Value>`. `LetBinding` evaluates the expression and inserts into env. Subsequent statements see the binding.

Scoping: Flat local scope. No nested blocks, no shadowing protection. A let that reuses a parameter name overwrites it.

### `if/else` as expressions

Grammar: `if_else_expr = { "if" ~ expression ~ "then" ~ expression ~ "else" ~ expression }` — the else branch is itself an expression, so `else if ... then ... else ...` works via natural nesting.

Keywords `if`, `then`, `else` excluded from `IDENT` via negative lookahead.

AST: `Expr::IfElse(Box<Expr>, Box<Expr>, Box<Expr>)` — condition, then-branch, else-branch.

Interpreter: evaluate condition, call `as_bool()` to coerce, evaluate the matching branch.

### Supporting changes

- `Value::Bool(bool)` — new runtime value
- `as_bool()` coercion: Bool→self, Float→!=0.0, String→!empty, Unit→false
- Comparison operators in expressions: `BinOp::Gt/Lt/Ge/Le/Eq` returning `Value::Bool`
- Arithmetic `*` and `/` added to expression `binop`
- `BOOL_LITERAL` — `true`/`false` keywords

### Design constraints (learned from failed first attempt)

1. `CONTAINS_KW` must NOT be in expression `binop` — conflicts with rule `contains_condition`
2. `IDENT` must exclude `if`/`then`/`else`/`let`/`return`/`true`/`false`
3. `parse_pattern_body` must drill into `let_binding`/`return_stmt` child pairs to find `expression`
4. `if_else_expr` must be first in `unary_expr` alternation for correct priority

## Consequences

### Positive
- Patterns can perform multi-step computation with `let`
- Patterns can branch conditionally with `if/else/else if`
- `if/else` as expression composes with `let` naturally
- Full backward compatibility — all existing .mlog programs unchanged

### Neutral
- Flat scoping (no nested blocks)
- Broad `as_bool()` coercion
- No operator precedence hierarchy — all binops at same level

## Contract Test

`examples/p5_let_if.mlog`:
```mlog
pattern Evaluate(score: Float) -> String {
  let grade = if score > 0.9 then "excellent"
              else if score > 0.7 then "good"
              else "needs work"
  let message = "Result: " + grade
  return message
}
entity score: Float = 0.85
flow Main { input: Float = score -> Evaluate -> output }
```
Expected: `Result: good`

## Files Changed

| File | Delta |
|------|-------|
| `src/grammar.pest` | +16/-3 — let_binding, return_stmt, if_else_expr, BOOL_LITERAL, *, /, comparisons, IDENT exclusions |
| `src/ast.rs` | +13 — Statement::LetBinding, Expr::BoolLit, Expr::IfElse, BinOp::Gt/Lt/Ge/Le/Eq |
| `src/parser.rs` | +55/-5 — parse_binop(), if_else_expr, BOOL_LITERAL, let_binding/return_stmt in pattern_body |
| `src/interpreter.rs` | +77/-35 — Value::Bool, as_bool(), mutable eval_statements, IfElse eval, comparison eval_binop |
| `examples/p5_let_if.mlog` | Contract test |
| `examples/p5_let_if.expected` | Expected output |
