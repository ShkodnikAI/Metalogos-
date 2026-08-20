# ADR-0064: SmartRouter scope limited to interpreter path (not VM)

**Status:** Accepted
**Date:** 2026-08-20
**Context:** Наряд №4

## Decision

SmartRouter integration (Наряд №4) targets **only the interpreter execution path** (`src/interpreter/`). The bytecode VM path (`src/vm.rs`) is explicitly **out of scope** for this наряд.

## Evidence

### Default execution path is interpreter

`mlog run file.mlog` calls `metalogos::run_program()` (lib.rs:60):

```rust
let mut interp = interpreter::Interpreter::new();
interp.set_base_dir(base_dir);
let output = interp.run(declarations)?;
```

`mlog serve` hardcodes `ServeBackend::Interpreter` in 4 places:

- `server.rs:526`: `backend: ServeBackend::Interpreter, // set after build_state returns`
- `server.rs:2006`: `backend: ServeBackend::Interpreter,`
- `server.rs:2021`: `Ok(_) => ServeBackend::Interpreter,  // fallback`
- `server.rs:2022`: `Err(_) => ServeBackend::Interpreter, // default`

VM path activates ONLY for:
1. `.mbc` bytecode files: `mlog run file.mbc` → `run_bytecode()`
2. Explicit env: `METALOGOS_SERVE_BACKEND=vm mlog serve`

### VM path has separate LLM calling

`src/vm.rs:2104-2180`: `Vm::call_llm()` calls `create_llm_backend()` directly.
This is a known gap, documented in ADR-0076 (VM dispatch paths).

## Justification

1. **All production usage goes through interpreter.** VM serve backend is
   experimental (test_n40_backend_env_default_is_interpreter proves default).
2. **VM lacks `llm {}` declaration processing.** The compiler doesn't
   compile LlmConfig declarations into bytecode. Wiring SmartRouter into
   VM requires: (a) compiling llm config to bytecode, (b) storing router
   in Vm struct, (c) updating Instruction::LlmCall dispatch. This is a
   separate наряд.
3. **Risk of scope creep.** Narjad №4's contract is: "SmartRouter built
   but not connected to call_llm()". Fixing it for interpreter is the
   minimal viable fix. VM is a follow-up.

## Consequences

- `mlog run .mlog` with `llm {}` declaration → SmartRouter active ✅
- `mlog serve` (default) → SmartRouter active ✅
- `mlog run .mbc` → legacy `create_llm_backend()` (no SmartRouter)
- `METALOGOS_SERVE_BACKEND=vm` → legacy `create_llm_backend()`

Follow-up наряд needed for VM path integration.
