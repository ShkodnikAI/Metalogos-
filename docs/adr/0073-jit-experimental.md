# ADR-0073: JIT backend status — declared experimental

**Date:** 2026-08-01
**Status:** accepted
**Context:** Наряд №33 Block 4

## Problem

The JIT backend (`src/jit.rs`, 280 lines) is a proof-of-concept that was
started during Phase 4.3 but never completed. Its current state:

- All JIT tests in `tests/jit_golden.rs` are `#[ignore]` with comment
  "VM Unimplemented (JIT not yet integrated)".
- The `run_jit()` helper returns `Err("JIT not available")` for all inputs.
- `Vm::with_jit()` was never implemented.
- No Cranelift or other JIT compiler is integrated.
- The code is essentially a scaffold with a comment about future Cranelift use.

## Decision

**JIT is declared experimental and removed from the main development path.**

1. **No work on JIT** until a deliberate decision to resume it.
2. **All JIT tests remain `#[ignore]`** — they serve as specification
   for what a future JIT should do.
3. **`src/jit.rs` is kept** as documentation of the intended design.
4. **Block 4 cross-check test** (`tests/crosscheck_backends.rs`) compares
   only interpreter vs VM. JIT is not included.

## VM vs Interpreter: current discrepancies

Cross-check of 58 golden examples: **37 match, 21 don't.**

### VM errors (6 programs crash)

| Program | Error |
|---------|-------|
| actor_potential.mlog | `undefined builtin: map` |
| dag_demo.mlog | `dag_phases: argument 0 must be List, got Float` |
| dept_schema.mlog | `undefined builtin: db_insert` |
| p30_db_params.mlog | `undefined builtin: query_scalar` |
| p30_slice.mlog | `slice() requires List as first argument` |
| p5_modules.mlog | `qualified calls not yet supported in bytecode` |

Root causes:
- **Missing builtins in VM dispatch table**: `map`, `db_insert`, `query_scalar`
- **Incorrect type coercion in VM**: `dag_demo` passes Float where List expected
- **Missing features**: `slice` on non-list, qualified imports

### Output mismatches (15 programs)

| Program | TW output | VM output | Likely cause |
|---------|-----------|-----------|--------------|
| p11_context_loading_fixed | Full context text | Stripped context | `__load_context` not in VM |
| p12_context_auto | Full context text | Stripped | Same as above |
| p12_context_literal | Full context text | Stripped | Same as above |
| p1_entity_store | "0.9" | "" | `find` returns stub in VM |
| p30_assign_mut | "1" | "0" | Reassignment not fully in VM |
| p30_scope_let | Full text | Empty | `let` scoping differs in VM |
| p5_each | "2 4 6 8 10" | "" | `each` loop not in VM |
| p5_self_host_lexer | Parsed tokens | "" | `each` + `split_tokens` not in VM |
| p5_while | "5 4 3 2 1" | "" | `while` loop not in VM |
| p7_env | "Port= Name=" | "" | `env()` result handling in VM |
| skill_index_tiered | "core_skill_a" | "" | `__*` internal builtins |
| v05_file_io | "file I/O works!" | "" | `read_file` not in VM |
| v05_if_else | "повышенная" | "38" | String comparison in VM |
| v05_integration | Full string | "3" | String ops differ |
| v05_kv_memory | "test_value" | "true" | kv_store semantics |

Root causes:
- **Missing VM opcodes**: `each`, `while`, `read_file`, `__load_context`
- **Type system differences**: string comparison, kv_store return type
- **Scope handling**: `let` in VM has different scoping rules

## Consequences

- Cross-check test (`crosscheck_tw_vs_vm_all_golden`) is green with
  threshold ≥ 30 (currently 37/58 pass).
- VM has significant feature gaps: loops, file I/O, string ops, scoping.
- Fixing VM discrepancies is a separate work item, not part of Block 4.
- JIT remains on hold indefinitely.
