# ADR-0011: Type Inference, Error Recovery, and Branch Overlap Detection

**Status:** Partially Implemented — multi-error reporting (done, see
`src/semantic.rs::AnalysisResult` with `errors: Vec<SpannedError>` since
наряда №165); type inference and branch overlap detection deferred — no
code in tree as of v0.18.0; revisit if/when typed flows return (see
ADR-0107 Float-only decision).

**Date:** 2026-05-31
**Milestone:** Phase 2

---

## Context

Phase 1 (ADR-0009) introduced semantic analysis with single-error reporting and no type inference.
The analyzer stopped at the first error and could not detect type mismatches between flow pipelines
and pattern parameters. Three limitations were identified:

1. **No type inference.** A Float flowing into a String-annotated pattern was not caught.
2. **Single error reporting.** Analysis stopped at the first error, requiring fix-compile cycles.
3. **No branch overlap detection.** Flow branches with overlapping conditions (e.g., `> 0.8` and
   `< 0.9`) were accepted silently, potentially hiding unreachable branches.

The user requirement: "если паттерн Shout(s: String) -> String, а flow передаёт в него Float,
semantic analysis должен поймать это до исполнения."

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| Hindley-Milner type inference | ML, Haskell, Rust | Full inference, but complex to implement |
| Constraint-based type checking | C++ templates, TypeScript | Practical balance of inference and annotation |
| Error recovery (continue after error) | Rust compiler, clang, GCC | Reports multiple errors per pass; better developer UX |
| Dead code detection | LLVM, GCC -Wunused | Flags unreachable branches/statements |
| Interval analysis for branches | static analysis tools | Detects overlapping switch/branch ranges |

## Decision

### 1. Expression Type Inference

Expression types are inferred statically and tracked through the flow pipeline:

```
source → type(source) → [step₁: type check against param] → type(step₁.return) → ... → output
```

**Inference rules:**

| Expression | Inferred Type |
|---|---|
| `StringLit("...")` | `String` |
| `FloatLit(42.0)` | `Float` |
| `Ident(name)` | Entity's declared type |
| `FieldAccess(base.field)` | Field type from struct definition |
| `FnCall(pattern_name, ...)` | Pattern's declared return type |
| `FnCall(builtin_name, ...)` | Builtin's static return type |
| `BinaryOp(+, String, String)` | `String` |
| `BinaryOp(+,-,*,/, Float, Float)` | `Float` |

**Type compatibility rules for flow-to-pattern binding:**

| Flow Type | Param Type | Compatible? | Reason |
|---|---|---|---|
| `String` | `String` | Yes | Identical |
| `Float` | `String` | **No** | Primitive mismatch |
| `Fluid` | anything | Yes | Lazy collapse at runtime |
| `Struct` (generic) | `Message` | Yes | `find()` returns generic struct |
| `Message` | `Payload` | **No** | Different struct types |
| `Unknown` | anything | Yes | Conservative: can't determine |

### 2. Error Recovery

Instead of returning on the first error, the analyzer accumulates all errors and warnings
into separate `Vec<String>` collections:

```rust
pub struct AnalysisResult {
    pub errors: Vec<String>,   // Prevent execution
    pub warnings: Vec<String>, // Do not prevent execution
}
```

**Format:**
- Single error: the error message directly
- Multiple errors: `"N errors:\n1: ...\n2: ..."` (numbered list)

This change required converting all `check_expr` and `check` methods from returning
`Result<(), String>` to pushing errors into the accumulator. Entities are still collected
even when they have type errors, to avoid cascading "undefined entity" follow-up errors.

### 3. Branch Overlap Detection

For each `branch_def` block, all pairs of branches on the same field are checked for
overlapping numeric ranges.

**Algorithm:** Each branch condition is converted to a half-interval on the real number line:

| Operator | Interval |
|---|---|
| `> X` | (X, +∞) exclusive |
| `>= X` | [X, +∞) inclusive |
| `< Y` | (-∞, Y) exclusive |
| `<= Y` | (-∞, Y] inclusive |
| `== V` | [V, V] inclusive |

Two half-intervals overlap if their intersection is non-empty. The intersection
is computed as `lo = max(lo₁, lo₂)`, `hi = min(hi₁, hi₂)`. If `lo < hi`, overlap.
If `lo == hi`, overlap only if both intervals include the boundary point.

**Output:** Warnings are non-blocking. They are prepended to the program output:
```
warning: overlapping branches 'high' and 'medium' in flow 'Main' step 'Route'
ESCALATE
```

## Rationale

- **Why not Hindley-Milner?** Full HM inference requires unification variables, let-polymorphism,
  and a separate type resolution pass. Metalogos has explicit type annotations on patterns and
  entities, making forward-propagation through the flow pipeline sufficient for Phase 2.
- **Why error recovery over "fail fast"?** A program with 5 typos shouldn't require 5 separate
  compile cycles. Reporting all errors at once is standard practice in production compilers.
- **Why interval-based overlap?** Branch conditions in Metalogos are simple comparisons on a single
  field. Interval arithmetic is the simplest correct analysis for this shape. Full constraint
  solving (SMT) would be overkill.
- **Why are warnings in the output?** Metalogos doesn't have a separate stderr channel in the
  golden test framework. Prepending warnings to the output keeps the test infrastructure simple
  while making warnings visible and testable.

## Impact

- **`src/semantic.rs`**: Major rewrite — `AnalysisResult` struct, `Context` gains `entity_types`,
  `struct_fields`, `pattern_return_types`, `learnable_return_types`, `errors`, `warnings` fields.
  Added `infer_type()`, `types_compatible()`, `check_branch_overlap()`, `ranges_overlap()`,
  `half_interval()`. All `check` methods now accumulate errors instead of returning early.
- **`src/codegen.rs`**: Return type changed to `Result<(Program, Vec<String>), String>` to
  propagate warnings.
- **`src/lib.rs`**: `run_program()` prepends warnings to the output string.
- **`examples/m2_triage.expected`**: Updated to include the new overlap warning between
  `medium` and `low` branches (correctly detected).
- **New test pairs:** `p2_type_mismatch.mlog/.error`, `p2_multi_errors.mlog/.error`,
  `p2_overlap_branches.mlog/.expected`
- **Backward compatible.** All 9 existing golden tests and 2 existing error tests pass.
  The m2_triage expected output was updated to include the newly-detected warning.
