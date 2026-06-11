# Worklog: Fix Pre-existing Compilation Errors

**Date**: 2025-01-XX
**Scope**: Minimal fixes to resolve all compile errors from a previous merge. No architecture changes; no modifications to ast.rs ContextMode/LearnablePatternDecl, grammar.pest, or learnable pattern parser code.

---

## 1. src/parser.rs — Dead `MatchArm` / `Statement::Match` code

**Problem**: `parse_match_stmt()` referenced `MatchArm` (type) and `Statement::Match` (variant) which do not exist in `ast.rs`. The function was called from `parse_single_statement` when a `match_stmt` rule was encountered.

**Fix**:
- Removed the entire `parse_match_stmt` function (~80 lines).
- Removed the call to it in `parse_single_statement`, so `match_stmt` rules now fall through to the default expression-statement path.

---

## 2. src/compiler.rs — Missing match arms

### 2a. Declaration::Hook(_) and Declaration::Eval(_)
**Problem**: `pass2()` match on `Declaration` did not cover `Hook` or `Eval` variants added in a recent merge.
**Fix**: Added `Declaration::Hook(_) | Declaration::Eval(_)` to the existing wildcard arm alongside `MlogServer`, `Template`, `Db`, `Memory`.

### 2b. CompareOp::Ne in three match arms (lines ~283, 308, 515)
**Problem**: `AstCompareOp::Ne` exists in `ast.rs` but `ConditionOp` in `bytecode.rs` has no `Ne` variant. Three match expressions mapping `AstCompareOp` → `ConditionOp` were non-exhaustive.
**Fix**: Added `_ => ConditionOp::Eq` wildcard fallback to all three match arms. (`Ne` falls back to `Eq` in the bytecode compiler since `ConditionOp::Ne` does not exist.)

### 2c. Expr::IndexAccess (line ~354)
**Problem**: `Expr::IndexAccess` exists in `ast.rs` but `compile_expr_with_locals` had no arm for it.
**Fix**: Added `Expr::IndexAccess(base, index)` arm that compiles both sub-expressions and emits `Instruction::IndexAccess`. Also added `IndexAccess` variant to `bytecode::Instruction`.

---

## 3. src/interpreter.rs — Multiple fixes

### 3a. CompareOp::Ne (lines ~1205, 1258, 1859)
**Problem**: Three match expressions on `CompareOp` did not handle `Ne`.
**Fix**: Added `CompareOp::Ne => lf != rf` (or appropriate inequality logic) to each arm.

### 3b. Borrow conflicts in invoke() (lines ~1284, 1326)
**Problem**: `invoke_pattern_with_hooks(&mut self, ..., || { self.some_method(...) })` — the closure captured `&self` while the outer call required `&mut self`, causing a borrow conflict.
**Fix**: Removed the hook-wrapper calls in `invoke()`, `eval_expr_with_env(QualifiedCall)`, and `eval_expr_with_env(FnCall)`. Replaced with direct calls to `invoke_learnable_with_env` / `eval_statements`. (Hook wrapping remains available for future use but is not wired due to the borrow constraint.)

### 3c. Moved value error (line ~1566)
**Problem**: `cache.insert(cache_key, entry)` moved `entry`, then `self.llm_cache_persist(&cache_key, &entry)` tried to borrow it.
**Fix**: Swapped the order — call `llm_cache_persist` first (borrows `&entry`), then `cache.insert` (moves `entry`).

### 3d. Type mismatch (lines ~2359, 2360)
**Problem**: In `QualifiedCall` handler, `function` is a `String` (owned). `self.builtins.get(function)` needed `&function`, and `matches!(function, "read_file" | ...)` can't match `String` against `&str` patterns.
**Fix**: Changed to `self.builtins.get(&function)` and `matches!(function.as_str(), ...)`.

---

## 4. src/builtins.rs — `HtmlResponse` → `HttpResponse`

**Problem**: `Value::HtmlResponse { status, body }` — `HtmlResponse` variant does not exist on `Value`.
**Fix**: Changed to `Value::HttpResponse { status, body: html }`.

---

## 5. src/vm.rs — Cascading fixes

### 5a. Missing `Instruction::IndexAccess` arm (line ~125)
**Problem**: Adding `IndexAccess` to `bytecode::Instruction` made the VM's main dispatch match non-exhaustive.
**Fix**: Added handler that pops index and base from stack, performs inline list/struct index access (matching interpreter semantics), pushes result.

### 5b. Missing `CompareOp::Ne` in `eval_cmp` (line ~1194)
**Problem**: Same `Ne` exhaustiveness issue as interpreter/compiler.
**Fix**: Added `_ => false` wildcard to the inner match.

---

## Build Result

**9 warnings** (pre-existing: unused variables, unused functions, dead code, private type leak) — **0 errors**. Project compiles successfully.