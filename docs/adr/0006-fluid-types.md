# ADR 0006: Phase 1 — Fluid Types: Lazy Collapse of Type Superpositions

**Status:** Accepted
**Date:** 2026-05-31
**Milestone:** Phase 1 — Type System (Fluid Types)

## Context

METALOGOS is an AI-native language where uncertainty is a first-class concept. The
README introduces "Fluid Types" — values that exist in a superposition of
concrete types, each annotated with a confidence score. The question is: when
does such a superposition collapse to a specific type, and what happens when
confidence is insufficient?

The `metalogos-language-semantics` skill provides the design guidance:

> Fluid = размеченное объединение вариантов + вектор уверенностей. Коллапс **ленивый,
> в точке использования**: операция/правило/ветвь, требующая конкретного типа,
> форсирует выбор варианта с максимальной уверенностью (или ошибку soft-failure,
> если ниже порога).

Prior art includes: gradual typing (Siek–Taha), refinement/liquid types, tagged
unions (sum types), and probabilistic types. The semantics skill explicitly warns
against building "настоящую вероятностную типизацию" on day one — this is an MVP.

The contract program is `examples/p1_fluid_types.mlog`:
```mlog
fluid x = Float[42.0][0.9] or String["answer"][0.1]

pattern Double(n: Float) -> Float { return n + n }

flow Main { input: Float = x -> Double -> output }
```

**Done when:** `mlog run p1_fluid_types.mlog` prints `84` (Fluid collapses to
Float variant with confidence 0.9, pattern doubles it).

## Decision

### Syntax

Fluid values are declared with the `fluid` keyword:

```mlog
fluid name = TypeName[value][confidence] or TypeName[value][confidence] ...
```

Each branch specifies: the type name, a concrete value in brackets, and a
confidence score (0.0..1.0) in brackets. Multiple branches are separated by `or`.

The grammar rule:
```pest
fluid_decl   = { FLUID_KW ~ IDENT ~ "=" ~ fluid_branch ~ ("or" ~ fluid_branch)* }
fluid_branch = { type_name ~ LBRACKET ~ expression ~ RBRACKET ~ LBRACKET ~ FLOAT_LITERAL ~ RBRACKET }
```

### Runtime representation

At the AST level, a `FluidDecl` contains a list of `FluidVariant` structs, each
with a `type_name: String`, `value: Expr`, and `confidence: f64`.

At runtime, the interpreter stores `Value::Fluid(Vec<FluidValueVariant>)` where
each `FluidValueVariant` holds an already-evaluated concrete `Value` alongside
its type name and confidence. This means the superposition is materialized at
declaration time — all variant values are computed eagerly, but the *choice*
of which variant to use is deferred.

### Lazy collapse semantics

Collapse is **lazy** — it does not happen when the fluid value is declared, but
only when the value is used in a context that requires a specific type. The
collapse trigger points are:

1. **Pattern invocation** — when a Fluid value is passed as an argument to a
   pattern or learnable pattern that declares typed parameters. The interpreter
   binds arguments to parameters via `bind_and_collapse`, which calls
   `maybe_collapse(arg, param.type_name)` for each argument.

2. **Future extension points** — binary operations on Fluid values, rule
   condition evaluation, and flow input type checking are natural candidates
   for future collapse triggers.

The `maybe_collapse` algorithm:
- If the value is not Fluid, return it unchanged.
- If the value is Fluid, find the variant whose `type_name` matches the
  required type and has the highest confidence.
- If that variant's confidence >= `COLLAPSE_THRESHOLD`, return its concrete
  value.
- Otherwise, return `Value::Unit` (soft-failure — no crash, no exception).

### Collapse threshold

The threshold is a compile-time constant: `COLLAPSE_THRESHOLD = 0.1`.

This is deliberately permissive for the MVP — it means most reasonable
confidence scores (>= 0.1) will successfully collapse. The threshold exists
to handle the edge case where a variant exists but has near-zero confidence,
signaling that the type is essentially a guess.

**Rationale for 0.1:** In practice, LLM confidence scores and heuristic
confidence values rarely fall below 0.1 unless the model is completely uncertain.
A higher threshold (e.g., 0.5) would cause legitimate low-confidence but
correct collapses to fail unnecessarily.

**Growth path:** Make the threshold configurable per-context (e.g., `flow Main
{ collapse_threshold = 0.5 ... }`) or per-declaration. This requires a design
decision about scope and inheritance — deferred to Phase 1 iteration.

### Soft-failure

When collapse fails (no matching variant, or confidence below threshold), the
interpreter returns `Value::Unit` instead of panicking. This follows the
established "soft-failure instead of exceptions" principle from the
`metalogos-language-semantics` skill. The calling context receives `Unit` and
can decide how to handle it — flow branches can test for it, patterns can
propagate it, etc.

### Display semantics

When a Fluid value is printed (e.g., as flow output), the `Display` impl
shows the highest-confidence variant's value. This is a pragmatic choice for
the MVP — a more informative display (showing all variants with confidences)
would be useful for debugging but is not needed for the contract test.

### What Phase 1 Fluid Types do NOT include

1. **Confidence propagation** — when a pattern receives a Fluid input and
   produces output, the output's confidence is not computed from the input's
   confidence. The semantics skill says: "выход несёт min (или произведение)
   уверенностей входов" — this is explicitly called out as a heuristic, not
   yet implemented. Output confidence is a Phase 1 follow-up.

2. **Automatic type coercion in variants** — if a Fluid has `String["42"]`
   and needs to collapse to Float, the interpreter does not attempt to parse
   the string as a number. Each variant's value must be already of the correct
   type. Type coercion is a Phase 1 follow-up.

3. **Probabilistic type inference** — no Bayesian or Dempster-Shafer reasoning
   is attempted. The confidence scores are treated as opaque annotations.
   Per the semantics skill: "Не делай настоящую вероятностную типизацию."

4. **Fluid-to-Fluid operations** — binary operations between two Fluid values
   are not supported. If both operands are Fluid, the binary operation will
   fail with a type mismatch error. This is acceptable for the MVP.

## Consequences

- Fluid Types introduce the first form of runtime type-level uncertainty to
  METALOGOS. A value can genuinely be "possibly a number, possibly a string,"
  and the language handles this gracefully.
- The lazy collapse design means zero overhead for non-Fluid values — the
  `maybe_collapse` function immediately returns non-Fluid values without
  any matching or comparison.
- The soft-failure guarantee is maintained: Fluid collapse never crashes the
  interpreter. Degraded confidence produces `Unit`, which propagates cleanly
  through the rest of the computation.
- M1 through M5 tests remain green — no regressions. The Fluid type is purely
  additive: new declaration type, new value variant, new collapse logic.
- The contract test proves the full cycle: `fluid` declaration → pattern
  invocation with typed parameter → lazy collapse → correct computation.
- Dead code was eliminated: the old `invoke_learnable` method (which lacked
  collapse support) was replaced by `invoke_learnable_with_env` (which
  receives pre-collapsed arguments).
