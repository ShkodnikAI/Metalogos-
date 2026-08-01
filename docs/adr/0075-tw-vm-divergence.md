# ADR-0075: TW vs VM divergence list (21 cases)

**Date:** 2026-08-01
**Status:** accepted
**Context:** Наряд №34 Block 1

## Problem

The cross-check test (`tests/crosscheck_backends.rs`) runs all 58 `.mlog`/`.expected`
pairs through both the tree-walking interpreter (TW) and the bytecode VM, then
compares outputs.

Result: **37 match, 21 diverge** (15 mismatches + 6 VM errors + 0 TW errors).

The original threshold (`>= 30`) was too loose — it allowed regression in a
third of the VM without detection. This ADR documents every divergence so each
can be investigated and resolved.

## Categories

| Category | Meaning | Count |
|---|---|---|
| `Mismatches` | Both backends ran but produced **different** output | 15 |
| `VM errors` | VM raised an error (missing builtin or unsupported construct) | 6 |
| `TW errors` | Interpreter raised an error where VM worked | 0 |

## Full list — Mismatches (15)

### 1. p11_context_loading_fixed.mlog
- **TW:** Full context (3 memorized lines) + prompt
- **VM:** Prompt only (no context)
- **Cause:** VM does not execute `memorize` / `recall` / `learnable pattern`
  with `context: recall()` — the memory subsystem is not wired into VM bytecode
  emission.

### 2. p12_context_auto.mlog
- **TW:** Full context (2 memorized lines) + prompt
- **VM:** Prompt only (no context)
- **Cause:** Same as #1 — `context: auto` in learnable patterns requires the
  memory subsystem which the VM does not support.

### 3. p12_context_literal.mlog
- **TW:** "You are helpful.\nGreet the user."
- **VM:** "Greet the user."
- **Cause:** VM does not handle `context: "literal"` in learnable patterns.
  The context line is silently dropped during compilation.

### 4. p1_entity_store.mlog
- **TW:** "0.9"
- **VM:** "()"
- **Cause:** The `rule` / `find()` system for entity store queries is not
  implemented in the VM. `find("Message", "urgency", "gt", 0.5)` returns
  empty (or null) in the VM, and `GetUrgency` gets null urgency.

### 5. p30_assign_mut.mlog
- **TW:** "1"
- **VM:** "0"
- **Cause:** VM does not propagate `let mut` + reassignment (`x = 1.0`)
  correctly. The value stays at the initial `0.0`. Mutation semantics differ
  between the interpreter's environment model and the VM's register model.

### 6. p30_scope_let.mlog
- **TW:** "text=[IZ-BLOKA] handled=1"
- **VM:** "text=[] handled=0"
- **Cause:** Scope semantics for `let` inside `if/then/else` blocks. The TW
  interpreter uses function-level scope (no block scope), per ADR `p30_scope_let`.
  The VM appears to implement block scope, so `let text = "IZ-BLOKA"` inside
  `if` creates a local binding that is discarded on exit. This is a
  **semantic difference** that must be aligned to the documented behavior.

### 7. p5_each.mlog
- **TW:** "2 4 6 8 10"
- **VM:** "" (empty)
- **Cause:** VM does not implement the `each item in items { ... }` loop
  construct. The pattern compiles but the loop body is never executed.

### 8. p5_self_host_lexer.mlog
- **TW:** Full tokenized output (KEYWORD, IDENT, OPERATOR, NUMBER lines)
- **VM:** "" (empty)
- **Cause:** Same root cause as #7 and #9 — VM does not support `while` loops.
  This self-hosted lexer relies entirely on nested `while` loops, so the VM
  produces no output at all.

### 9. p5_while.mlog
- **TW:** "5 4 3 2 1"
- **VM:** "" (empty)
- **Cause:** VM does not implement `while` loops. The bytecode compiler either
  skips the construct or emits no loop bytecode.

### 10. p7_env.mlog
- **TW:** "Port=8080 Name=fosved"
- **VM:** "()"
- **Cause:** VM does not implement the `env()` builtin. The call returns null,
  and string concatenation with null yields "()".

### 11. skill_index_tiered.mlog
- **TW:** "core_skill_a"
- **VM:** "()"
- **Cause:** VM does not implement `skill_index` declarations or the
  `resolve_skill_index()` builtin. The struct result is null.

### 12. v05_file_io.mlog
- **TW:** "file I/O works!"
- **VM:** "" (empty)
- **Cause:** VM does not implement file I/O builtins (`write_file`,
  `read_file`, `delete_file`). The pattern returns empty.

### 13. v05_if_else.mlog
- **TW:** "повышенная — наблюдать"
- **VM:** "38"
- **Cause:** VM evaluates the `if / else if / else` chain incorrectly. With
  `temp = 38.0`, the condition `temp > 37.5` should be true and return the
  correct string. Instead, the VM outputs `38.0` (the raw input), suggesting
  the `if/else if` chain falls through or the pattern returns the input instead
  of executing the branches.

### 14. v05_integration.mlog
- **TW:** "DLROW SOGOLATEM OLLEH (из 3 слов)"
- **VM:** "3"
- **Cause:** VM returns only the word count `3.0` instead of the full
  transformed string. Likely the string builtins chain (`trim`, `upper`,
  `reverse`, `split`, `length`) partially works but `reverse` or concatenation
  fails, and the fallback path returns `count` instead of `result`.

### 15. v05_kv_memory.mlog
- **TW:** "test_value"
- **VM:** "true"
- **Cause:** VM's `kv_exists()` returns the string "true" instead of a boolean
  that feeds into the `if` truthiness check. Alternatively, `kv_get` returns
  "true" instead of "test_value". The KV store builtins are partially
  implemented but with incorrect return types.

## Full list — VM errors (6)

### 16. actor_potential.mlog
- **Error:** `undefined builtin: map`
- **Cause:** The `map()` higher-order builtin (apply pattern to list elements)
  is not registered in the VM's builtin table.

### 17. dag_demo.mlog
- **Error:** `dag_phases: argument 0 must be List, got Float`
- **Cause:** `dag_phases()` is registered in the VM but receives incorrect
  argument type. The list-of-structs literal `[{ id: "...", depends_on: [...] }]`
  is compiled as a different type (Float?) in the VM, suggesting struct literal
  compilation is broken.

### 18. dept_schema.mlog
- **Error:** `undefined builtin: db_insert`
- **Cause:** `db_insert()` is not registered in the VM. The `schema/table` DDL
  system and database builtins were added in Наряд №30 only for the interpreter.

### 19. p30_db_params.mlog
- **Error:** `undefined builtin: query_scalar`
- **Cause:** `query_scalar()` is not registered in the VM. Same root cause as
  #18 — DB builtins from Наряд №30 are interpreter-only.

### 20. p30_slice.mlog
- **Error:** `slice() requires List as first argument`
- **Cause:** `slice()` is registered in the VM but the argument type check fails.
  Likely the list literal `["a", "b", "c", "d", "e"]` is compiled incorrectly
  in the VM, producing a non-List value.

### 21. p5_modules.mlog
- **Error:** `compile: qualified calls not yet supported in bytecode`
- **Cause:** The `import std/string as str` / `import std/math as m` syntax with
  qualified calls (`str.trim()`, `m.abs()`) is explicitly not supported by the
  VM compiler. The compiler rejects these at compile time.

## TW errors

None. The tree-walking interpreter successfully executes all 58 examples.
This confirms the interpreter is the more complete backend.

## Root cause summary

| Root cause | Cases | Priority |
|---|---|---|
| `while` / `each` loops not in VM | #7, #8, #9 | P1 |
| `let mut` reassignment broken in VM | #5 | P1 |
| Scope semantics differ (`let` in blocks) | #6 | P1 |
| Memory subsystem (memorize/recall/context) not in VM | #1, #2, #3 | P2 |
| String builtins partially broken in VM | #13, #14, #15 | P1 |
| `env()` not in VM | #10 | P2 |
| `skill_index` / domain builtins not in VM | #11 | P2 |
| File I/O builtins not in VM | #12 | P2 |
| `rule` / `find()` entity store not in VM | #4 | P2 |
| `map()` not in VM | #16 | P1 |
| Struct literal compilation broken in VM | #17, #20 | P1 |
| DB builtins not in VM | #18, #19 | P2 |
| Qualified calls (`import ... as`) not in VM | #21 | P2 |

## Test output (verbatim)

```
═══ Cross-check TW vs VM: 37/58 passed ═══

── Mismatches (15): ──
  ✗ p11_context_loading_fixed.mlog: TW="Relevant context:\n- Metalogos is an AI-native programming language\n- Metalogos uses Pest parser for grammar definitions\n- Metalogos supports fluid types with confidence propagation\n\nAnswer the question about Metalogos." VM="Answer the question about Metalogos."
  ✗ p12_context_auto.mlog: TW="Relevant context:\n- Metalogos is an AI-native programming language\n- Metalogos uses Pest parser for grammar definitions\n\nAnswer the question." VM="Answer the question."
  ✗ p12_context_literal.mlog: TW="You are helpful.\nGreet the user." VM="Greet the user."
  ✗ p1_entity_store.mlog: TW="0.9" VM="()"
  ✗ p30_assign_mut.mlog: TW="1" VM="0"
  ✗ p30_scope_let.mlog: TW="text=[IZ-BLOKA] handled=1" VM="text=[] handled=0"
  ✗ p5_each.mlog: TW="2 4 6 8 10" VM=""
  ✗ p5_self_host_lexer.mlog: TW="KEYWORD: entity\nIDENT: count\nOPERATOR: :\nIDENT: Float\nOPERATOR: =\nNUMBER: 42.0" VM=""
  ✗ p5_while.mlog: TW="5 4 3 2 1" VM=""
  ✗ p7_env.mlog: TW="Port=8080 Name=fosved" VM="()"
  ✗ skill_index_tiered.mlog: TW="core_skill_a" VM="()"
  ✗ v05_file_io.mlog: TW="file I/O works!" VM=""
  ✗ v05_if_else.mlog: TW="повышенная — наблюдать" VM="38"
  ✗ v05_integration.mlog: TW="DLROW SOGOLATEM OLLEH (из 3 слов)" VM="3"
  ✗ v05_kv_memory.mlog: TW="test_value" VM="true"

── VM errors (6): ──
  ✗ actor_potential.mlog: VM: undefined builtin: map
  ✗ dag_demo.mlog: dag_phases: argument 0 must be List, got Float
  ✗ dept_schema.mlog: VM: undefined builtin: db_insert
  ✗ p30_db_params.mlog: VM: undefined builtin: query_scalar
  ✗ p30_slice.mlog: slice() requires List as first argument
  ✗ p5_modules.mlog: compile: qualified calls not yet supported in bytecode
```

## Resolved in Наряд №34 (8 cases)

| # | File | What was fixed |
|---|---|---|
| 5 | p30_assign_mut.mlog | `let mut` reassignment fixed in VM |
| 6 | p30_scope_let.mlog | Scope semantics aligned to interpreter |
| 12 | v05_file_io.mlog | File I/O builtins connected to VM |
| 13 | v05_if_else.mlog | if/else chain fixed in VM |
| 14 | v05_integration.mlog | String builtin chain fixed |
| 15 | v05_kv_memory.mlog | KV store return types fixed |
| 16 | actor_potential.mlog | `map()` registered in VM |
| 20 | p30_slice.mlog | `slice()` argument type check fixed |

## Resolved in Наряд №35 (4 cases)

| # | File | What was fixed |
|---|---|---|
| 7 | p5_each.mlog | `eval_cmp` in VM now handles String comparisons (was Float-only) |
| 8 | p5_self_host_lexer.mlog | Same `eval_cmp` fix — string comparison in loop conditions |
| 9 | p5_while.mlog | Same `eval_cmp` fix — leading space eliminated |
| 17 | dag_demo.mlog | `MakeStruct` and `Contains` added to `execute_code()` (were only in `run()`) |

## Remaining (9 cases) — documented remainder

| # | File | Category | Root cause | Complexity |
|---|---|---|---|---|
| 1 | p11_context_loading_fixed | Mismatch | Memory subsystem (memorize/recall) not wired into VM bytecode | High |
| 2 | p12_context_auto | Mismatch | Memory subsystem `context: auto` not in VM | High |
| 3 | p12_context_literal | Mismatch | Learnable `context: "literal"` not compiled in VM | Medium |
| 4 | p1_entity_store | Mismatch | `rule` / `find()` entity store not in VM | High |
| 10 | p7_env | Mismatch | Flow source expression compiler does not support BinOp — `env()` works in entities but concatenation in flow input falls through to `Ident()` | Medium |
| 11 | skill_index_tiered | Mismatch | `skill_index` / `resolve_skill_index` not registered in VM | Medium |
| 18 | dept_schema | VM error | `db_insert` declared in BUILTIN_REGISTRY but no handler implemented | High |
| 19 | p30_db_params | VM error | `query_scalar` declared in BUILTIN_REGISTRY but no handler implemented | High |
| 21 | p5_modules | VM error | `import ... as` qualified calls explicitly not supported by VM compiler | Medium |

## Consequences

1. This ADR is the reference for all TW vs VM divergence work.
2. Each case must be resolved individually; bulk fixes are not acceptable.
3. The threshold in `crosscheck_backends.rs` is now set to `>= 49` — the exact
   current match count. It must only increase.
4. `assert!(mismatches.is_empty())` remains commented until all remaining
   mismatches and VM errors are resolved.
5. Additional finding: `execute_code()` (used by pattern calls from flow
   pipelines) was missing 14 instruction handlers that were only in `run()`.
   `MakeStruct` and `Contains` were added; remaining unhandled instructions
   are top-level only (FlowExec, RegisterPattern, etc.) and correctly stay
   in `run()`.

