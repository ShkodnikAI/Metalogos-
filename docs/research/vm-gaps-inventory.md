# VM Gaps Inventory — closing the Match / BlockIfElse / match_expr gaps

> **Naryad #293** (issue #356, P0/adr — VM_COMPLETE). Source: external Metalogos audit 2026-09-13. Verified on main `fdfbfb7` (post-#292 merge).
> **Owner decision 2026-09-14**: supersedes the "Do not implement…" caveats of ADR-0105 — staged closure of the gaps; the default flip (ADR-0088) remains behind the gates: parity 100% + full crosscheck + soak + real load.

## 1. Known gaps (verified on main `fdfbfb7`)

### 1.1. `Match` statement (compiler.rs:1373)

```rust
Statement::Match { .. } => {
    return Err("compile: Match statement not yet supported in VM bytecode \
         (use tree-walking interpreter)"
        .into());
}
```

**Semantics** (REFERENCE §3.4 + `src/interpreter/execution.rs`): `match expr { arm* else? }` — 4 kinds of arm:
- exact: `"val" then { stmts }`
- prefix: `starts_with "pre" then { stmts }`
- substring: `contains "sub" then { stmts }`
- compare: `> expr then { stmts }` (compares the scrutinee with expr, any of `>`/`<`/`>=`/`<=`/`==`/`!=`)
- `else { stmts }` — fallback.

Match as a **statement** — executes the selected arm for its side-effects, the result is not used. Match as an **expression** (`let x = match y { ... }`) — see §1.3.

**Cost of closure** (per the precedent of naryad #91 — TryEval):

| Component | Estimate | Details |
|---|---|---|
| New bytecode instruction | ~30 lines | `Match { scrutinee_code, arms: Vec<MatchArm>, else_code: Option<Vec<Instruction>> }` in `src/bytecode.rs`. `MatchArm { kind: MatchArmKind, value_code: Vec<Instruction>, body_code: Vec<Instruction> }`. `MatchArmKind { Exact, StartsWith, Contains, Compare(BinOp) }`. |
| Compiler (compile Match statement) | ~80 lines | In `compile_statement`: for each arm — compile scrutinee + compile value-expr + compile body. Match expression → Const(value) + branch-to-arm on the result. |
| Compiler (compile Match as expression for `let`/`return`) | ~80 lines | Similar, but the body returns a Value — a new opcode `MatchReturn` is needed, or a stack-based Result via the existing `Return` + a scope-aware approach. |
| VM dispatch — execute_arm | ~60 lines | In both dispatch loops (`run` for the main program, `execute_route_code` for route handlers): for each arm — check the condition, if it matches — execute the body; if all fail — execute the else (or return Unit). |
| Tests | ~150 lines | New `tests/naryad_<N>_vm_match.rs` — all 4 arm kinds + else + match as statement + match as expression + recursion inside an arm body. |
| crosscheck_backends.rs — remove `p_match_switch.mlog` exclusion | 1 line | `if name == "p_match_switch.mlog" { continue; }` → remove. |
| **Total** | **~400 lines** | Precedent naryad #91 (TryEval) — ~250 lines. Match is more complex (4 arm kinds, compare with 6 operators) — ~400. |

### 1.2. `Expr::BlockIfElse` (compiler.rs:902)

```rust
Expr::BlockIfElse { .. } => {
    return Err("compile: block if/else expression not yet supported \
         in VM bytecode (use tree-walking interpreter)"
        .into());
}
```

**Semantics** (REFERENCE §3.4 + `src/ast.rs:1483`): `if cond { stmts } else { stmts }` as a **value** (in `let`/`return`/argument position). The value is the last expression in the selected branch (Unit if there is no non-Unit expr).

**Important**: `Statement::IfElseBlock` (block if/else as a statement) is **already supported** by the VM (naryad #129). Only the expression form is unsupported.

**Cost of closure**:

| Component | Estimate | Details |
|---|---|---|
| New bytecode instruction | ~15 lines | `BlockIfElse { cond_code: Vec<Instruction>, then_code: Vec<Instruction>, else_ifs: Vec<(Vec<Instruction>, Vec<Instruction>)>, else_code: Option<Vec<Instruction>> }`. Alternative: reuse `JumpIfFalse`/`Jump` + `Pop` — but the structured opcode is simpler. |
| Compiler (compile BlockIfElse expr) | ~50 lines | In `compile_expr_with_locals`: for each branch — compile condition + compile body (the last stmt returns the value via a `Return`-free path). Last-expression-in-block → value semantics (in TW this works via `eval_block` — the VM needs an equivalent). |
| VM dispatch | ~40 lines | In both loops: evaluate cond → jump-to-matching-branch → execute → leave the value on the stack. |
| Tests | ~100 lines | New `tests/naryad_<N>_vm_block_if_else_expr.rs` — simple/nested/else-if/no else (Unit). |
| **Total** | **~205 lines** | Less than Match — no arm-kind variability, but value-semantics-of-last-stmt is harder (TW eval_block) |

### 1.3. `match_expr` (`let x = match y { ... }`) — TW-only, naryad #173b

`match_expr` is a `Match` statement used in a `let`/`return` position. It is in fact **part of gap 1.1** — if the Match statement is closed in the VM, its expression form must be closed too. The tasking of naryad #173b added match_expr to TW only; for the VM it will be closed automatically when Match-as-expression is implemented in gap 1.1.

**Cost**: included in §1.1 (compiler Match as expression, ~80 lines) — no separate work is required.

## 2. Hidden gaps — inventory (grep `unimplemented`/`not yet supported`)

### 2.1. compiler.rs — 2 explicit gaps

```
$ grep -nE "unimplemented|not yet supported|not supported|TODO\(vm\)|FIXME\(vm\)" src/compiler.rs
902:                return Err("compile: block if/else expression not yet supported \
1374:                    return Err("compile: Match statement not yet supported in VM bytecode \
```

Only two gaps — both from §1. There are **no** other `unimplemented`/`not yet supported` entries in compiler.rs.

### 2.2. vm.rs — 0 explicit gaps

```
$ grep -nE "unimplemented|not yet supported|not supported|TODO\(vm\)|FIXME\(vm\)" src/vm.rs
(empty)
```

`vm.rs` contains no `unimplemented!()` or `not yet supported` — all unused opcode arms are handled via a `=> {}` no-op or `panic!("unknown opcode: {:?}")` (for genuinely unknown opcodes).

### 2.3. crosscheck_backends.rs — 3 exclusions

```
$ grep -n "continue;" tests/crosscheck_backends.rs | head -5
```

| File | Reason | Gap |
|---|---|---|
| `p_match_switch.mlog` | Exercises `match` statement | §1.1 — close Match → remove the exclusion |
| `p118_collection_utils.mlog` | `unique`/`chunk`/`sort` results via string `+` — heterogeneous types | VM `eval_binop` rejects heterogeneous; TW auto-coerces. A separate binop coercion gap. |
| `reflex_math.mlog` | `random_seed`/`random` (TW-only — VM has no PRNG state); Bool→String formatting ("true" in TW, "1" in VM) | 2 gaps: PRNG state + Bool→String formatting parity. |

### 2.4. Hidden gaps (derived from §2.3)

After closing §1.1 (Match), 2 hidden gaps remain:
- **Binop coercion** — heterogeneous List + String concatenation. VM eval_binop strict, TW lenient. Cost: ~80 lines (loosen eval_binop + 6-10 contract tests).
- **PRNG state** — `random_seed`/`random` — TW-only. Cost: ~50 lines (add `RandomState` to the Vm struct, seed propagation, deterministic mode for tests).
- **Bool→String formatting** — `"true"` vs `"1"`. Cost: ~10 lines (formatting in vm.rs).

**Total hidden gaps**: 3 (binop coercion, PRNG, Bool→String).

## 3. Precedent naryad #91 — TryEval

Naryad #91 closed `Expr::Try` (`try expr`) in the VM. Pattern:
1. New bytecode instruction `TryEval(Vec<Instruction>)` in `src/bytecode.rs` (10 lines — enum variant).
2. Compiler: `Expr::Try { expr: inner, .. }` → compile inner into a sub-vec, emit `TryEval(inner_code)` (5 lines).
3. VM dispatch — both loops (`run` + `execute_route_code`): for `TryEval(inner_code)` — evaluate inner, catch error → push Unit; success → push value (10 lines per loop = 20 lines).
4. Tests: `tests/naryad_91_*` (~30 lines — success path + error path).
5. crosscheck_backends.rs — remove exclusion (1 line).

**~250 lines total for one instruction**. Match is more complex (4 arm kinds × 6 operators × branch logic) → ~400 lines. BlockIfElse is simpler (no arm kinds) but value-semantics-of-last-stmt is harder → ~205 lines.

## 4. Total cost of closure

| Gap | Cost (LOC) | Naryads |
|---|---|---|
| §1.1 Match statement + expression | ~400 | 1 naryad (~#294) |
| §1.2 BlockIfElse expression | ~205 | 1 naryad (~#295) |
| §2.4 Binop coercion (heterogeneous types) | ~80 | 1 naryad (~#296) |
| §2.4 PRNG state | ~50 | 1 naryad (~#297) |
| §2.4 Bool→String formatting parity | ~10 | mini-naryad (can be merged with PRNG) |
| crosscheck_backends.rs cleanups | 3 lines | in each of the above |
| **Total** | **~745 LOC** | **~4 naryads** |

All 4 naryads follow the precedent of naryad #91 in structure: instruction + compiler + VM dispatch + tests. They require no ADR (a VM extension, not new language semantics — the ADR-0105 caveat is lifted by the owner decision).

## 5. The "strategic need" criterion (for flipping the ADR-0088 default)

The default flip `METALOGOS_SERVE_BACKEND=interpreter` → `vm` — only under **all** of the conditions:

1. **Parity 100%** — all 3 crosscheck exclusions lifted (p_match_switch, p118_collection_utils, reflex_math). This means: all 4 gaps from §4 are closed.
2. **Full crosscheck green** — `tests/crosscheck_backends.rs` without a single `continue;` (except negative-test contracts like p50_unknown_fn, p2_wrong_types — which are designed-to-fail).
3. **Soak period** — 1 sprint (≈2 weeks) of FOSVED running on the VM backend in staging, without panic/regression. Currently FOSVED runs on TW; VM is opt-in for experiments only.
4. **Real load** — benchmark on a production-class .mlog file (≥2000 lines, with LLM calls, DB, vision — a representative FOSVED workload). The VM must show ≥2× latency improvement or equivalent latency with a memory/CPU win.

Without ALL four conditions the default flip is not done — ADR-0088 `Implemented (default remains interpreter)` stays.

## 6. Stage plan (owner decision 2026-09-14 — supersedes the ADR-0105 caveats)

**Stage 0** (this naryad — #293): research + ADR-0141 + inventory (this document). Zero code.

**Stage 1** (naryads #294–#297): closing the 4 gaps per the precedent of naryad #91. Each — a separate naryad, a separate PR, separate tests + crosscheck exclusion removal. Order: Match (the biggest effect — p_match_switch) → BlockIfElse → binop coercion → PRNG + Bool→String.

**Stage 2**: parity gate — after all 4 naryads, `tests/crosscheck_backends.rs` without `continue;` exclusions for VM-uncovered constructs. If parity is 100% — proceed to Stage 3.

**Stage 3**: soak — FOSVED on the VM in staging for 1 sprint. Without panic/regression → proceed to Stage 4.

**Stage 4**: real-load benchmark — representative FOSVED workload on the VM. ≥2× latency improvement or equivalent latency with a memory/CPU win.

**Stage 5** (only if Stages 2-4 are green): flip the ADR-0088 default `interpreter` → `vm`. A separate ADR (new number — `0142` or higher). The opt-out via `METALOGOS_SERVE_BACKEND=interpreter` is preserved for back-compat.

## 7. Risks

1. **Match expression value-semantics** — TW `eval_block` returns the last non-Unit value. VM bytecode has no concept-of-block-as-value; the needed pattern: the last stmt in the block → `SetLocal`/`Push`. Risk: subtleties with `if-then-else` inside a block (statement vs expression).
2. **BlockIfElse vs Statement::IfElseBlock overlap** — the statement form already works, the expression form does not. The compiler must distinguish the context (`let x = if ...` vs `if ... { stmts }`). In TW this distinction is made in execution.rs; the VM compiler must make the same distinction.
3. **Binop coercion** — loosening strict typing in VM `eval_binop` may break existing VM tests (which rely on strict). All heterogeneous-binop tests must be reworked — `assert!(result.is_err())` → `assert_eq!(result, ...)`.
4. **PRNG determinism** — the VM must have a deterministic mode for tests (seed propagation via `Vm::set_random_seed`). If the random state is global — races between request handlers in serve.
5. **Soak regressions** — the VM may have edge cases under production load that are not caught on examples. 1 sprint is the minimum period; if regressions appear — extend it.

## 8. Alternatives

- **Do not close the gaps, keep the VM experimental** (the original position of ADR-0105). The owner decision 2026-09-14 supersedes this caveat, so the alternative is rejected.
- **Close all gaps in one mega-naryad** — rejected: review-load risk, regression-batch. Per the precedent of naryad #91 — one naryad per gap.
- **Default flip without soak** — rejected: ADR-0088 already recorded the "static checks vs production" risk. Soak is mandatory.

## 9. What NOT to do in this naryad (#293)

- Do not compile Match/BlockIfElse (Stage 1 — naryads #294–#297).
- Do not change the backend default (Stage 5 — after Stages 2-4).
- Do not "quietly" reopen ADR-0105 — only an explicit supersede by the owner (done 2026-09-14, recorded in this document + ADR-0141).
