# ADR-0156: TW/VM/JIT label parity — LabelJoin/SinkCheck in the bytecode

**Status:** Proposed (amended 2026-11-12 by №949 — §5, the label is a property of the VALUE; the owner's explicit «принято» returns Accepted — канон §13.14 п.6)
**Date:** 2026-09-15
**Naryad:** №328 (issue #422, plan v2 §16.3 / Волна 1 · Фаза 1)
**Supersedes:** the reserved stub written by №319 (booking note in git history)

## 1. Decision

The static gates (№323 inference, №325 sink clearance, №327 decisions) are the SSOT of every verdict. №328 lowers that knowledge into the bytecode so the VM runtime twin agrees by construction and any divergence is a loud error:

- `Instruction::LabelJoin { dst, src }` — joins the runtime label of `src` into `dst` (componentwise, ADR-0154 §2.4). A `src` starting with `@` names a №316 Source builtin; the VM seeds the runtime label from the same mapping the static pass uses (Secret sources → `(private, trusted)`, every other source → `(public, untrusted)`).
- `Instruction::SinkCheck { fn_name, arg, line }` — the runtime twin of the №325 gate: the runtime label of `arg` must clear the sink (`public`; exec additionally refuses untrusted). A violation is a distinct runtime error `[SINK_CLEARANCE_RUNTIME]` + an audit event line on stderr.

The compiler lowers source-backed `let`/assignments into `LabelJoin` and every sink call site (identifiers and direct-source arguments) into `SinkCheck` — in both compile paths (top-level statements and pattern bodies).

> **№780 note (audit-23.10 A1, wave 62, 2026-10-23):** the sentence above was the CONTRACT, not the fact — until №780 the bodies (pattern/route) compiled their sink sites WITHOUT the armed `SinkCheck` (`compile_pattern_body_with_locals` had no `emit_armed_sink_checks` on `ExprStmt`/`Return`) AND the VM inner loop dropped the `LabelJoin`/`SinkCheck` arms into the wildcard tail, so the server code (serve route bodies) never executed the runtime twin. №780 closes BOTH halves: the compiler arms the body sinks (symmetric to `compile_stmt_with_locals`), the VM inner loop executes the label instructions from the SAME executors as the main loop (the deny path №392 included), and the main-only instructions answer LOUD in a body (`compiler/VM skew`) — the split is pinned by the exhaustive enumeration test (a new `Instruction` variant without a class is a compile error).

## 2. The dispatch-gap rule

The JIT compiler is not in the tree yet. `bytecode::is_jit_eligible` is the SSOT predicate the future dispatcher must consult: label instructions are explicitly OUTSIDE the JIT-eligible class (arithmetic-only). When a JIT dispatcher appears, it is required to reject label-bearing functions with a distinct error naming this ADR — never to skip them silently. Pinned by test.

## 3. Parity matrix (Фаза 1)

| Verdict | TW (run) | VM (compile+run) | JIT |
|---|---|---|---|
| Static gate verdicts (№325/№327) | Error at compile | Error at compile | label fns excluded from the eligible class — explicit reject |
| Runtime label tracking | env-tracked by the №323 contracts | `LabelJoin` over the runtime label env | not eligible (explicit error, future) |
| Sink verdict at runtime | via the same compile gate | `SinkCheck` runtime twin, loud divergence error | not eligible (explicit error, future) |

Golden verdicts: the run and compile paths agree on both rejecting and accepting programs (pinned by test).

## 4. Verification (№328)

`cargo test naryad_328` — 8 tests: the dispatch-gap predicate, the VM runtime twin, runtime source labels matching the static mapping, the componentwise join, golden run/compile verdict agreement, compiler emission into the bytecode, no-stub grep.

## 5. Appendix (№949): the label is a property of the VALUE — the runtime second line of defense

Added by naryad №949 (gh#1916, the total audit 12.11, P0 — ADR BEFORE code; the owner decision 12.11: revising accepted records is permitted). **This appendix is PROPOSED — the owner's explicit «принято» turns it Accepted (канон §13.14 п.6); the implementation naryad is №952.**

### 5.1 The honest boundary of the accepted form

The №328 `LabelJoin`/`SinkCheck` pair lowers STATIC knowledge into the bytecode: a runtime label env seeded from the same №316 mapping, a sink check against the same clearance. It is a projection of the static pass — where the static analysis is blind (the inter-pattern boundary, ADR-0154 §12), the runtime twin is blind too. The audit 12.11 demonstrated this on the published binary: the label does not survive calls, returns, collections, or the flow machinery as a property of the VALUE.

### 5.2 The amendment (the plan's п. 1.7 — «метка в `Value`»)

The label becomes a property of the VALUE in BOTH executors:

- **The value carries its label.** A `Value` produced by a source, received through a call boundary, returned from a pattern, read out of a collection (list/struct/map), or crossing a flow step, carries the lattice label (ADR-0154 §12's summary is what the static line computes; the value label is what the RUNTIME observes).
- **Propagation is by construction**: calls propagate argument labels into the callee's parameter values; returns propagate the return value's label; collections propagate element/field labels; the flow machinery propagates step-to-step. Join is componentwise (ADR-0154 §2.4) at every combination point.
- **The sink checks the VALUE's label.** `SinkCheck` (and the TW runtime gate) reads the label of the arriving VALUE — not of the static binding. A sink refusal fires even when the static analysis was wrong (the second line of defense): the refusal class stays `[SINK_CLEARANCE_RUNTIME]` (no new words — the №385 frozen vocabulary).
- **The static line stays first.** The static gates remain the SSOT of every verdict and the BLOCKING leak-suite negatives; the runtime check is the safety net that converts a static miss into a loud refusal at the sink instead of a silent leak. The dual-backend parity contract of this ADR applies unchanged: the value-label propagation and the sink value-check behave IDENTICALLY in TW and VM (the crosscheck lane covers value-label shapes).

### 5.3 The overhead budget and the way it is measured

The value label rides the existing runtime label machinery (the №328 env → per-value metadata); the budget: the label propagation must not add more than the №376 dispatch threshold (+50%) to the execution time of the examples corpus + the office `app.mlog` under BOTH backends, measured in the №952 PR (the same corpus and harness form as №376: the median over N runs, cold and warm). A measured excess is a loud No-Go for the default-on form — the key `profile legacy { value_labels: off }` is the explicit opt-out for the transition window, removed together with the №949 interproc key after the window.
