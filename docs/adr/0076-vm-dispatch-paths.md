# ADR-0076: VM Dispatch Path Coverage

**Status:** accepted
**Date:** 2026-08-01
**Context:** Наряд №36, Block 1

## Problem

`vm.rs` has two execution paths:

1. **`run()`** — top-level program execution.
2. **`execute_code()`** — nested execution (pattern bodies, map(), flow steps).

These paths handle different sets of bytecode instructions. 17 instructions
are intentionally top-level-only (`Adapt, ExecuteRules, FlowExec, Forget,
Halt, JumpIfLow, ListLen, MakeFluid, MakeList, Memorize, Mutate, Pop,
RegisterLearnable, RegisterPattern, Relate, StartsWith, StoreGlobal`).

The remaining 27 instructions are shared between both paths.

**The risk:** `execute_code()` has a catch-all `_ => { ip += 1; }` that
silently skips unhandled instructions. If a new instruction is added to
the enum and handled in `run()` but forgotten in `execute_code()`, the
discrepancy manifests only in specific scenarios — exactly how `MakeStruct`
and `Contains` were discovered in №35.

## Decision

**Coverage test (option b).**

A test `vm_dispatch_coverage` in `tests/vm_golden.rs` maintains two
constant lists:

- `ALL_INSTRUCTIONS`: every variant in `bytecode::Instruction`.
- `TOP_LEVEL_ONLY_INSTRUCTIONS`: instructions intentionally only in `run()`.

The test verifies every instruction in `ALL_INSTRUCTIONS` is accounted for.
Adding a new variant to the enum without updating these lists causes a
test failure. This is cheaper than a common dispatcher (option a) and
provides equivalent protection.

## Top-level-only instructions (17)

Registration: `RegisterPattern`, `RegisterLearnable`
Pipeline: `FlowExec`, `ExecuteRules`
Memory: `Memorize`, `Forget`
Adapt/Relate/Mutate: top-level learning operations
Struct/List: `MakeList` (not yet in execute_code)
Fluid: `MakeFluid`
List ops: `ListLen`, `Pop`, `StartsWith`
Control: `Halt`, `JumpIfLow`
Variables: `StoreGlobal`

**Note:** `Contains` and `MakeStruct` were added to `execute_code()` in
Наряд №35. They were previously top-level-only.

## Consequences

- Adding a new `Instruction` variant without updating `ALL_INSTRUCTIONS` → test failure.
- Adding an instruction to `ALL_INSTRUCTIONS` without it being in `TOP_LEVEL_ONLY` means it MUST be handled by both paths.
- The silent catch-all in `execute_code()` remains as a safety net, but the test prevents new instructions from falling through it unnoticed.
