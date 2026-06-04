# Phase 2 Report: Type Inference, Error Recovery, Branch Overlap Detection

**Commit:** `27d0eb1`
**Date:** 2026-05-31
**Status:** Closed. All 13 tests green (9 golden + 4 error).

---

## What was built

### 1. Type Inference through Flow Pipeline

The semantic analyzer now infers expression types statically and tracks them through
flow pipelines. When a flow step invokes a pattern, the analyzer checks whether the
current flowing type is compatible with the pattern's first parameter type.

**Contract (`p2_type_mismatch.mlog`):**
```mlog
entity greeting: Float = 42.0
pattern Shout(s: String) -> String { return upper(s) + "!" }
flow Main { input: Float = greeting -> Shout -> output }
```
**Result:** Error before execution — `type mismatch: pattern 'Shout' expects String, but flow provides Float`.

**Type compatibility rules:**
- Identical types → compatible
- `Fluid` → compatible with everything (lazy collapse at runtime)
- Generic `Struct` (from `find()`) → compatible with any user-defined struct
- Different primitives (String vs Float) → **incompatible**
- Different user-defined structs (Message vs Payload) → **incompatible**

### 2. Error Recovery — All Errors in One Pass

The analyzer now collects ALL errors and warnings per pass, instead of stopping at the
first error. The output format:

- Single error: the message directly
- Multiple errors: `"N errors:\n1: ...\n2: ..."` (numbered list)

**Contract (`p2_multi_errors.mlog`):**
```mlog
entity bad: FakeType = { text: "oops" }
pattern Shout(s: String) -> String { return upper(s) + "!" }
flow Main { input: String = nonexistent -> Shout -> output }
```
**Result:** `"2 errors:\n1: unknown type 'FakeType' for entity 'bad'\n2: undefined entity 'nonexistent'"`.

Entities with type errors are still collected into scope (to avoid cascading follow-up errors).

### 3. Unreachable / Overlapping Branch Detection

For each `branch_def` block, all pairs of branches on the same field are checked for
overlapping numeric ranges using interval arithmetic.

**Contract (`p2_overlap_branches.mlog`):**
```mlog
Route {
    high (msg.urgency > 0.8) -> Escalate
    medium (msg.urgency < 0.9) -> Queue
}
```
**Result:** Warning (non-blocking): `warning: overlapping branches 'high' and 'medium' in flow 'Main' step 'Route'`.

The interval arithmetic converts each comparison to a half-interval on the real line
(e.g., `> 0.8` → `(0.8, +∞)`) and checks for non-empty intersection.

**Side effect:** Existing `m2_triage.mlog` correctly flagged that `medium (< 0.8)` and
`low (< 0.4)` overlap — the `low` branch is indeed unreachable since `medium` catches
everything first. The `.expected` file was updated.

---

## Files changed

| File | Change |
|---|---|
| `src/semantic.rs` | Major rewrite: `AnalysisResult`, type inference, error recovery, overlap detection |
| `src/codegen.rs` | Return type: `Result<(Program, Vec<String>), String>` — propagates warnings |
| `src/lib.rs` | Prepends warnings to program output |
| `examples/m2_triage.expected` | Updated with new overlap warning |
| `examples/p2_type_mismatch.mlog/.error` | New: type mismatch contract |
| `examples/p2_multi_errors.mlog/.error` | New: error recovery contract |
| `examples/p2_overlap_branches.mlog/.expected` | New: overlap detection contract |
| `docs/adr/0011-type-inference.md` | ADR documenting all decisions |

## Test suite: 13 pairs, all green

| # | Test | Type | Phase |
|---|---|---|---|
| 1 | m1_hello | golden | M1 |
| 2 | m2_triage | golden (with warning) | M2 |
| 3 | m3_classify | golden | M3 |
| 4 | m4_memory | golden | M4 |
| 5 | m5_adapt | golden | M5 |
| 6 | p1_fluid_types | golden | P1.0 |
| 7 | p1_confidence_propagation | golden | P1.2 |
| 8 | p1_entity_store | golden | P1.3 |
| 9 | p2_overlap_branches | golden (with warning) | P2 |
| 10 | err_undef | error | P1 |
| 11 | err_unknown_step | error | P1 |
| 12 | p2_type_mismatch | error | P2 |
| 13 | p2_multi_errors | error | P2 |

## ADR

- **ADR-0011 (type-inference):** Documents type inference rules, error recovery strategy,
  interval-based overlap detection, and prior art (Hindley-Milner, constraint-based, LLVM).
