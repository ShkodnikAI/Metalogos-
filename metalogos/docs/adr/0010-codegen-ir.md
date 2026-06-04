# ADR-0010: Codegen and IR (Intermediate Representation)

**Status:** Implemented (Phase 1 — AST wrapper)
**Date:** 2026-05-31
**Milestone:** Phase 1 (Closure), target Phase 4 (Bytecode VM)

---

## Context

The Metalogos interpreter directly walks the AST produced by the pest parser. This works but creates
a tight coupling between parsing and execution. Phase 4 plans to introduce a bytecode VM, which
requires an intermediate step between parsing and execution.

Question: what intermediate representation (IR) should Metalogos use, and how should the codegen
pipeline be structured?

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| AST as IR | Many tree-walking interpreters | Zero overhead, no serialization |
| ANF (A-Normal Form) | ML compilers, LLVM IR | All expressions named, enables optimization |
| SSA (Static Single Assignment) | LLVM, GCC | Enables register allocation, dead code elimination |
| Flat instruction list | CPython bytecode, JVM bytecode | Simple, serializable, VM-ready |
| High-level IR → Low-level IR | Cranelift (Rust), GraalVM | Multi-stage optimization |

## Decision

**Phase 1: Validated AST wrapper.** The IR is a thin struct wrapping the AST declarations after
semantic analysis:

```rust
// src/ir.rs
pub struct Program {
    pub declarations: Vec<Declaration>,
}
```

The codegen step validates via semantic analysis and wraps:

```rust
// src/codegen.rs
pub fn compile(decls: Vec<Declaration>) -> Result<Program, String> {
    semantic::analyze(&decls)?;
    Ok(Program { declarations: decls })
}
```

**Phase 4: Flat instruction buffer.** The IR will be replaced with:

```rust
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
}
```

Where `Instruction` is a tagged enum:
```rust
pub enum Instruction {
    LoadConst(usize),          // Push constant[addr] onto stack
    LoadVar(String),           // Push variable value onto stack
    StoreVar(String),         // Pop and store to variable
    GetField(String),         // Pop struct, push field value
    SetField(String),         // Pop value, struct; set field; push struct
    Call(String, u8),         // Call function with N args
    Add, Sub, Mul, Div,       // Binary ops
    Jump(usize),              // Unconditional jump
    JumpIfFalse(usize),       // Conditional jump
    Return,                   // Return top of stack
    CollapseFluid(String),    // Collapse fluid to type
    PropagateConfidence,      // Attach confidence to result
}
```

### Pipeline

```
Phase 1:  source → parse → [semantic] → codegen (wrap) → tree-walk interpreter
Phase 4:  source → parse → [semantic] → codegen (instructions) → bytecode VM
```

The `codegen::compile()` function is the boundary. In Phase 1, it wraps. In Phase 4, it translates.
The interpreter's public API (`run(declarations)`) stays the same; the internals change.

## Rationale

- **Why not a full IR in Phase 1?** A full instruction buffer requires implementing a stack machine,
  register allocation, and control flow lowering. This is substantial work that doesn't add user-facing
  value in Phase 1. The thin wrapper establishes the architectural boundary with minimal risk.
- **Why `semantic::analyze` inside `codegen::compile`?** Every valid IR must pass semantic analysis.
  Coupling them in `compile()` ensures that `ir::Program` is always valid — it's impossible to
  construct an invalid IR. This is the "make invalid states unrepresentable" principle.
- **Why not Cranelift/LLVM?** External IR frameworks add heavy dependencies and complexity. Metalogos
  is a small language; a custom lightweight IR is appropriate. Phase 4 may use Cranelift as a backend
  for native codegen, but the frontend IR will stay custom.

## Impact

- **`src/ir.rs`:** New module — `Program` struct
- **`src/codegen.rs`:** New module — `compile()` function
- **`src/lib.rs`:** Pipeline extended: `parse → codegen → interpret`
- **No interpreter changes.** The interpreter still walks AST types. Phase 4 replaces the
  interpreter's internals with a bytecode VM while keeping the `ir::Program` boundary.
- **Backward compatible.** All existing tests pass through the new pipeline unchanged.
