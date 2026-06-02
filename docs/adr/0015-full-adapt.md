# ADR-0015: Full Adapt (Mutate + Sandbox + Rollback)

**Status:** Implemented (Phase 2 Final)
**Date:** 2026-06-01
**Milestone:** Phase 2

---

## Context

Before this ADR, Metalogos had `adapt PatternName add_example("in", "out")` which
unconditionally appended a few-shot example to a learnable pattern's cache.
There was no way to:
1. Test whether a change actually improves the pattern's behavior
2. Roll back a bad change
3. Restrict what operations a mutation can perform

The metalogos-language-semantics skill specifies the full adapt contract:

> `mutate` меняют **только** few-shot набор `learnable`-паттерна (in-context).
> Каждая мутация: (1) применяется в **sandbox** (allow/forbid/timeout);
> (2) прогоняет тест-сьют; (3) **откатывается**, если метрика упала ниже порога.

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| In-context learning | GPT few-shot | No training, limited by context window |
| Git-style rollback | Git bisect/revert | Full state snapshots, overhead |
| A/B testing | Online experiments | Statistical confidence, live traffic |
| DSPy optimize | Khattab et al., 2023 | Programmatic prompt optimization |
| Sandbox (seccomp) | Docker, gVisor | Isolation, kernel-level enforcement |
| Mutate + rollback | M5 spec (this project) | Simple, pattern-scoped |

## Decision

### `sandbox` Declaration

Records a named sandbox configuration. For now, the sandbox is stored but not
enforced at runtime (enforcement deferred to Phase 3 when we have process isolation).

```mlog
sandbox test_env {
  allowed: [compute, read_data],
  forbidden: [network, write_permanent],
  timeout: 60
}
```

- **allowed**: list of permitted operation categories
- **forbidden**: list of blocked operation categories
- **timeout**: maximum execution time in seconds

### `mutate` Declaration

Performs a test-and-rollback cycle on a learnable pattern's few-shot cache:

```mlog
mutate Sentiment {
  add_example("terrible experience", "negative")
  rollback_if: accuracy < 0.9
}
```

Semantics:
1. Look up the learnable pattern by name (error if not found)
2. Save the current few-shot cache (for potential rollback)
3. **Replace** the few-shot cache with the new examples (not append — `adapt` appends,
   `mutate` replaces, simulating a full retrain)
4. Compute mock accuracy (0.95 for MockLlm; real ML backend in Phase 3)
5. Compare accuracy against the `rollback_if` threshold:
   - If passes → keep the new examples, log `[MUTATE] Name: accuracy=X, kept (>= threshold)`
   - If fails → restore the old few-shot cache, log `[MUTATE] Name: accuracy=X, rolled back (below threshold)`
6. If no `rollback_if` clause → keep unconditionally (dangerous, but allowed)

The `[MUTATE]` status message is prepended to the program output.

### `adapt` vs `mutate` — Key Difference

| | `adapt` | `mutate` |
|---|---|---|
| Operation | Append few-shot example | Replace entire few-shot set |
| Test | No | Accuracy check + optional rollback |
| Safety | None | Sandbox + rollback_if |
| Use case | Incremental improvement | Full retrain with evaluation |

### Limitations

1. **Mock accuracy always 0.95.** No real test suite execution yet. The accuracy
   comes from the ML backend (MockMlBackend) which returns a fixed value.
   Real accuracy measurement requires running examples through the pattern
   and comparing outputs to expected results — deferred to Phase 3.
2. **Sandbox not enforced.** The `sandbox` declaration is recorded but no
   operations are actually restricted. Enforcement requires process isolation
   (seccomp, namespace) which is beyond the interpreter's scope.
3. **No rollback history.** Only one level of undo (original cache). Chain
   mutations don't accumulate snapshots.
4. **Pattern-scoped only.** `mutate` only changes the few-shot cache of a
   single learnable pattern. It cannot modify rules, entity types, or other
   declarations (by design — see safety invariant below).

### Safety Invariant

From the metalogos-language-semantics skill:

> **Инвариант безопасности:** правила, помеченные как safety-critical, мутации
> трогать не могут. Это проверяется до применения мутации, а не после.

Currently all rules are mutable. Safety-critical marking is deferred to when
the rule system supports metadata annotations (Phase 3).

## Rationale

- **Why replace instead of append?** `adapt` already handles incremental
  few-shot addition. `mutate` represents a more dramatic operation —
  "retrain the model with this new dataset." The replace semantics make
  the rollback meaningful: you either fully accept the new training set
  or revert to the old one.
- **Why mock accuracy 0.95?** Deterministic golden test output. Real accuracy
  would vary between runs. The mock value is above most reasonable thresholds
  (0.9) to test the "kept" path, and below strict thresholds (1.0) to test
  the "rolled back" path.
- **Why not enforce sandbox now?** Process isolation (seccomp/namespace) is
  an OS-level concern that requires spawning child processes. The interpreter
  currently runs everything in-process. Sandbox enforcement requires
  refactoring the interpreter to support subprocess execution — Phase 3.

## Impact

- **`grammar.pest`:** New `sandbox_decl`, `mutate_decl`, `sandbox_body`,
  `mutate_body`, `ident_list`, `SANDBOX_KW`, `MUTATE_KW` keywords.
- **`ast.rs`:** `SandboxDecl`, `MutateDecl` structs, new Declaration variants.
- **`parser.rs`:** `parse_sandbox_decl()`, `parse_mutate_decl()`.
- **`interpreter.rs`:** `sandboxes: HashMap`, `mutate_log: Vec<String>`,
  `handle_mutate()` method, `take_mutate_log()`.
- **`lib.rs`:** Mutate log prepended to output.
- **Backward compatible.** All 6 existing golden tests pass.
- **New test:** `p2_full_adapt.mlog` — sandbox + mutate with kept status.
