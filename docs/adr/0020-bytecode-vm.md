# ADR 0020: Bytecode VM for METALOGOS

**Status:** Accepted
**Date:** 2026-06-01
**Phase:** 4.1

## Context

METALOGOS Phases 1-3 implemented a tree-walking interpreter (pest PEG parser → AST → semantic analysis → tree-walking execution). While correct, tree-walking interpretation has inherent performance limitations: every execution re-traverses the AST, function calls involve recursive interpreter dispatch, and there is no representation amenable to optimization passes.

Phase 4 introduces a bytecode compilation layer and stack-based virtual machine. This provides:
1. A compilation target separate from the AST, enabling future optimizations.
2. A stack-based execution model with explicit value management.
3. Support for METALOGOS-specific operations (confidence branching, Fluid collapse, memory).
4. Full parity with the tree-walking interpreter across all 11 golden test examples.

## Decision

### Architecture

Three new modules form the bytecode pipeline:

```
AST → compiler.rs → Program → vm.rs → Value (output)
```

**`bytecode.rs`** defines the instruction set (`Instruction` enum) and supporting types:
- `Program`: complete compiled program (globals, patterns, learnables, rules, main_code)
- `CompiledFn`: a compiled pattern with parameter types and instruction body
- `CompiledLearnableInfo`: LLM-backed pattern metadata
- `CompiledRule`: rule condition + assignment for the rule engine
- `CallFrame`: return address + base pointer for function calls

**`compiler.rs`** translates `Vec<Declaration>` into a `Program`:
- Two-pass compilation: pass 1 collects type info and assigns global slots; pass 2 generates instructions
- Expression compilation is context-aware: pattern parameters are compiled as `LoadLocal`, globals as `LoadGlobal`
- Import resolution mirrors the tree-walking interpreter (recursive file loading)
- Flow declarations are compiled into a single `FlowExec` macro instruction

**`vm.rs`** executes a `Program`:
- Stack-based dispatch loop with call frames for nested pattern invocations
- Pattern bodies are executed via `execute_code` (separate stack context)
- `invoke_step` handles flow pipeline steps with Fluid collapse and branch evaluation
- `execute_rules` implements the rule engine with name-based global lookup
- Memory operations (memorize/recall/forget) with decay and knowledge graph traversal
- LLM calls via the existing `llm::create_llm_backend()` trait

### Instruction Set

| Category | Instructions |
|----------|-------------|
| **Stack** | `Const`, `LoadGlobal`, `LoadGlobalByName`, `StoreGlobal`, `LoadLocal` |
| **Functions** | `RegisterPattern`, `RegisterLearnable`, `CallBuiltin`, `CallPattern`, `LlmCall`, `Return` |
| **Arithmetic** | `Add`, `Sub`, `Mul`, `Div` |
| **Comparison** | `Contains`, `CmpGt`, `CmpLt`, `CmpGe`, `CmpLe`, `CmpEq` |
| **Struct** | `MakeStruct`, `GetField` |
| **Fluid** | `MakeFluid`, `Collapse` |
| **Control** | `Jump`, `JumpIfNot`, `JumpIfLow` |
| **Memory** | `Memorize`, `Recall`, `Forget` |
| **Adapt/Mutate** | `Adapt`, `Relate`, `Mutate` |
| **Pipeline** | `FlowExec`, `ExecuteRules` |
| **Meta** | `Halt` |

### METALOGOS-Specific Opcodes

- **`JumpIfLow(threshold, target)`**: Confidence-based branching. Pops a value; if it's a Float below the threshold or a Fluid whose max confidence is below the threshold, jumps. Enables the `flow` branching syntax.
- **`Collapse(type_name)`**: Collapses a Fluid superposition to a concrete type. Finds the highest-confidence variant matching the type; returns Unit if below the collapse threshold (0.1). Matches the interpreter's `maybe_collapse` semantics.
- **`Memorize(priority)`** / **`Recall`** / **`Forget(days)`**: Memory store operations with priority, decay, and substring-based recall. Matches the interpreter's memory subsystem.
- **`FlowExec`**: A macro instruction carrying the flow definition (source expression, pipeline steps, branch definitions). The VM expands it internally by loading the source and stepping through the pipeline.
- **`ExecuteRules`**: A macro instruction that evaluates all registered rules in priority order, modifying global variables on condition match.

### VM Execution Model

```
main_code loop:
  dispatch instruction → modify stack/globals → advance IP
  CallPattern → push frame, execute pattern code, pop frame
  FlowExec → load source, invoke_step for each pipeline step
  ExecuteRules → eval conditions, modify globals
  Halt → return flow output
```

### Fluid Type Collapse

When calling a pattern with typed parameters, the VM collapses Fluid arguments to the parameter's declared type before binding. This matches the tree-walking interpreter's `bind_and_collapse` behavior. The collapse threshold is 0.1 (same as the interpreter).

## Consequences

### Positive
- **Full VM parity**: All 11 golden test examples produce identical output through both tree-walking and VM paths.
- **Separation of concerns**: Compilation and execution are cleanly separated, enabling future optimization passes.
- **METALOGOS-native opcodes**: Confidence branching, Fluid collapse, and memory operations are first-class VM instructions.
- **Zero warnings**: Clean compilation with no clippy warnings or dead code.

### Neutral
- The VM currently reuses the interpreter's `Value` type, `Builtins` registry, and `llm` backend. This is intentional for Phase 4.1 parity; a fully independent VM value representation is deferred to Phase 4.2+.
- FlowExec and ExecuteRules are "macro instructions" that carry their definitions inline. This simplifies the initial implementation; future phases may decompose these into sequences of primitive instructions.

### Future Work
- Phase 4.2: Decompose FlowExec into primitive instructions for optimization.
- Phase 4.3: Add a peephole optimizer (constant folding, dead code elimination).
- Phase 4.4: Self-hosting — compile the compiler itself to bytecode.
