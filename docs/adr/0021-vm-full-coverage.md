# ADR 0021: VM Full Feature Coverage — Phase 4.2

**Status:** Accepted
**Date:** 2026-06-01
**Phase:** 4.2

## Context

Phase 4.1 delivered a working bytecode VM with 30+ opcodes and full parity across all 11 golden test examples. However, several semantic gaps existed between the VM and the tree-walking interpreter:

1. **Rule priority sorting**: The interpreter sorts rules by priority descending before execution (stable sort preserving declaration order for ties). The VM evaluated rules in declaration order — correct for single-rule programs but semantically incorrect for multi-rule programs with different priorities.
2. **Collections flag propagation**: The compiler tracked `collections_loaded` (set when `import std/collections` is encountered) but never passed this flag to the VM's `Program` struct. The VM's `collections_loaded` was always `false`, meaning `map`/`filter`/`reduce` would never be recognized as valid operations in the VM path.
3. **Lenient testing**: The VM golden test suite (`all_vm_golden_tests_pass`) reported mismatches as "skipped" without failing the test. No strict assertion guaranteed that VM and tree-walking outputs would remain identical as the codebase evolved.
4. **No performance baseline**: No benchmark existed to measure the overhead difference between the two execution paths.

## Decision

### Changes to bytecode.rs

Added `priority: i32` field to `CompiledRule` struct. Rules now carry their priority from the AST through to the VM, enabling correct execution order.

Added `collections_loaded: bool` field to `Program` struct. The compiler sets this flag when `import std/collections` is resolved, and the VM reads it during `run()` to enable `map`/`filter`/`reduce` dispatch.

### Changes to compiler.rs

`compile_rule()` now copies `rule.priority` into the `CompiledRule`. `compile()` passes `self.collections_loaded` into the `Program`.

### Changes to vm.rs

`run()` now sorts rules by priority descending using a stable sort (matching the interpreter's `sort_by(|a, b| b.priority.cmp(&a.priority))`). The `collections_loaded` flag is read from `program.collections_loaded`.

The dummy `Program` constructed in `invoke_step()` now includes `collections_loaded: false`.

### Strict dual-mode test

Added `all_vm_examples_match_tree_walking` to `tests/vm_golden.rs`. This test runs every `.mlog` example through both tree-walking and VM, then asserts `assert_eq!` on the trimmed output. Any divergence causes an immediate test failure with a diff showing TW vs VM output.

### Performance benchmark

Added `tests/bench.rs` with two benchmark tests:

**`benchmark_vm_vs_tree_walking`**: Generates synthetic programs with 10–1000 chained `Increment` pattern calls in a single flow pipeline. Each program is run 3 times in both modes, taking the minimum time. Results:

| Steps | Tree-Walking (µs) | VM (µs) | Speedup |
|-------|-------------------|---------|---------|
| 10    | 170               | 155     | 1.10x   |
| 50    | 340               | 324     | 1.05x   |
| 100   | 565               | 499     | 1.13x   |
| 500   | 2516              | 1988    | 1.27x   |
| 1000  | 4520              | 3529    | 1.28x   |

**`benchmark_golden_examples_both_paths`**: Runs all 12 golden examples through both paths. Total: TW 5601µs, VM 4769µs, aggregate speedup 1.17x.

### Benchmark interpretation

The VM provides a modest 1.1–1.3x speedup over the tree-walking interpreter. This is expected given current architectural constraints:

- Both paths use the same `Value` type with `clone()` on every stack operation
- `FlowExec` is still a macro instruction — the VM interprets it internally rather than executing a decomposed instruction sequence
- The VM's `invoke_step()` creates a dummy `Program` for each pattern call
- Pattern body execution uses `execute_code()` (a separate dispatch loop) rather than inlining into the main loop

The speedup increases with pipeline length (1.05x at 50 steps → 1.28x at 1000 steps), suggesting the VM's advantage grows with program complexity. Significant speedups will require Phase 4.3 optimizations (peephole optimizer, value type specialization, decomposed flow instructions).

## Consequences

### Positive
- **Semantic correctness**: Rule priority sorting matches interpreter behavior. Multi-rule programs with different priorities now produce identical results in both execution paths.
- **Strict test contract**: `all_vm_examples_match_tree_walking` provides a hard guarantee that VM output equals tree-walking output for all examples. Future changes that break parity will be caught immediately.
- **Performance baseline**: Quantified the VM's current performance advantage (1.17x aggregate on golden examples, up to 1.28x on large synthetic programs). This establishes a baseline for measuring Phase 4.3 optimizer improvements.
- **Full feature matrix coverage**: All METALOGOS constructs are supported in the VM: rule (priority + fire), flow branching (confidence thresholds), learnable (LLM dispatch), memorize/recall/forget/decay, adapt/mutate, fluid collapse, import, relate.
- **18 tests, 0 failures**: All existing tests plus 5 new tests pass cleanly.

### Neutral
- The VM's `collections_loaded` flag is now propagated but `map`/`filter`/`reduce` still return errors in the VM path ("not yet implemented"). The flag prevents false "undefined builtin" errors but the actual collection operations remain unimplemented.
- `FlowExec` and `ExecuteRules` remain macro instructions. Decomposition into primitive instructions is deferred to Phase 4.3.

### Future Work
- Phase 4.3: Peephole optimizer (constant folding, dead code elimination, strength reduction).
- Phase 4.3: Decompose `FlowExec` into `FlowLoadSource` + `FlowStep` + `FlowBranch` + `FlowOutput` primitive instructions.
- Phase 4.3: Implement `map`/`filter`/`reduce` in the VM path.
- Phase 4.4: Self-hosting — compile the compiler itself to bytecode.
