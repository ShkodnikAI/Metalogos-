# Topic 2 — Blocks and control flow

**Status: normative (the №645 protocol).** Every norm below was probed on
BOTH backends before it was written; the conformance pairs prove the
record on every CI run. Where the probes found a cross-backend
divergence, there is NO norm — the finding lives in «Honest limits» and
needs its own repair naryad (the №645 → №651 lineage; three findings
were made by THIS topic's probe, see the limits).

The condition semantics (the truthy set, the refusal on composites) are
recorded ONCE — in [S-VAL-001](values.md) and
[S-VAL-013](values.md) — and are REFERENCED here, never re-stated.

### S-BLK-001 — The block value is its last expression

The value of a block (a pattern body, a branch body, a match arm body)
is the value of its LAST EXPRESSION statement; the let-bindings of the
preamble do not contribute.
**Anchors:** TW `eval_statements_cf` (src/interpreter/execution.rs); VM
the №370 value register (src/vm.rs:1572 `BeginValueExpr` — `KeepLastValue`
— `EndValueExpr`).
**Conformance:** `tests/conformance/sblk_001_block_value.mlog`.

### S-BLK-002 — An empty block is Unit

A block with no statements evaluates to `Unit`.
**Anchors:** TW `eval_statements_cf` (src/interpreter/execution.rs); VM
`BeginValueExpr`/`EndValueExpr` (src/vm.rs:1572/1588 — the register
starts as Unit).
**Conformance:** `tests/conformance/sblk_002_empty_block.mlog`.

### S-BLK-003 — if/else is an expression

The `if cond { … } else { … }` form is an expression whose value is the
value of the branch the condition selected (the condition semantics are
the S-VAL-013 condition set).
**Anchors:** TW `Expr::BlockIfElse` evaluation (src/interpreter/execution.rs); VM
the `Expr::BlockIfElse` compilation — the №370 value register over the
jump structure (src/compiler.rs, src/vm.rs:1572/1588).
**Conformance:** `tests/conformance/sblk_003_ifelse_value.mlog`.

### S-BLK-004 — A branch that leaves no value is Unit

A block-if with a false condition and NO else arm evaluates to `Unit`;
so does a selected branch whose body leaves no non-Unit value. (Unit is
falsy — the S-VAL-013 set — never a refusal.)
**Anchors:** TW `Expr::BlockIfElse` evaluation (src/interpreter/execution.rs); VM
the same compilation as S-BLK-003 (src/vm.rs:1572/1588 — «or stay Unit»).
**Conformance:** `tests/conformance/sblk_004_no_else_unit.mlog`.

### S-BLK-005 — The while condition is tested first

The `while` condition is tested BEFORE every iteration, including the
first; a falsy first condition runs the body zero times. The condition
truthy-set is the S-VAL-013 condition set — a composite/opaque/Fluid
condition refuses, it never silently skips the loop.
**Anchors:** TW the `Statement::While` arm (src/interpreter/execution.rs —
`cond_val.as_bool()?`); VM the `JumpIfNotCond` emission for the while
condition (src/compiler.rs, src/vm.rs).
**Conformance:** `tests/conformance/sblk_005_while_first_false.mlog`.

### S-BLK-006 — The loop runs exactly as long as the condition holds

The body runs once per satisfied test, in order, and the loop terminates
at the first unsatisfied test. A safety cap on the iteration count is an
implementation defense against non-termination, not part of the
semantics.
**Anchors:** TW the `Statement::While` arm and `WHILE_SAFETY_LIMIT`
(src/interpreter/execution.rs); VM the while compilation — the back-jump
and the after-loop patch (src/compiler.rs, src/vm.rs).
**Conformance:** `tests/conformance/sblk_006_while_count.mlog`.

### S-BLK-007 — `return` inside a value-channel arm is captured

A `return v` inside a value-channel arm body (an if/else or match arm
read for its value) does NOT abort the enclosing block: its value
REPLACES the arm's value — even with `v = Unit` — and execution
continues after the expression. (The №622 contract; `return` at the
statement level still returns from the pattern, which this norm does
not change.)
**Anchors:** TW the `eval_statements_with_mutability` flatten
(`ControlFlow::Return(v) => Ok(v)`, src/interpreter/execution.rs); VM
`Instruction::SetValueReg` (src/vm.rs:1593).
**Conformance:** `tests/conformance/sblk_007_return_capture.mlog`.

### S-BLK-008 — match is an expression; no match and no else is Unit

The `match` value is the selected arm's last expression; when nothing
matches and there is no `else` arm, the value is `Unit` (falsy in a
condition — the S-VAL-013 set).
**Anchors:** TW `Expr::MatchExpr` evaluation (src/interpreter/execution.rs:1646);
VM the `MatchExpr` compilation — `MatchTest` + the №370 value register
(src/compiler.rs, src/vm.rs).
**Conformance:** `tests/conformance/sblk_008_match_value.mlog`.

### S-BLK-009 — Block statements evaluate in the written order

The statements of a block evaluate one after another, top to bottom; the
observable state after the block reflects that sequence.
**Anchors:** TW `eval_statements_cf` (src/interpreter/execution.rs); VM the
compiled instruction list — the straight-line code between the jump
targets (src/compiler.rs, src/vm.rs `run`).
**Conformance:** `tests/conformance/sblk_009_block_order.mlog`.

### S-BLK-010 — An else-if condition is evaluated only when reached

The chain `if c1 { … } else if c2 { … } else { … }` evaluates `c2` only
when `c1` is falsy, and the final `else` only when every preceding
condition is falsy — an unreached condition is never evaluated (and, per
S-VAL-013, never refuses).
**Anchors:** TW the `Statement::IfElseBlock` arm — the lazy per-branch
evaluation (src/interpreter/execution.rs); VM the else-if chain
compilation comment («an else-if condition is only evaluated when
reached», src/compiler.rs) over the same `JumpIfNotCond` structure.
**Conformance:** `tests/conformance/sblk_010_lazy_elseif.mlog`.

### S-BLK-011 — `break` exits the nearest enclosing loop

A `break` inside a loop body (a `while`, an `each`, an `each_with_index`)
exits the NEAREST enclosing loop — at any nesting depth (through `if`
branches, match arm bodies, nested loops) the signal reaches the nearest
loop edge, the statements after that loop run next, and the outer loops
keep iterating. TW and VM behave identically.
**Anchors:** TW the `ControlFlow::Break` propagation — `eval_block!`
forwards the signal out of every nested block and the `While`/`Each` arms
absorb it (src/interpreter/execution.rs); VM the №658 loop-stack fixups —
the shared statement compiler registers a `Jump(0)` placeholder in the
NEAREST loop-stack entry and the loop compiler patches it to `after_loop`
from the POPPED entry (src/compiler.rs).
**Conformance:** `tests/conformance/sblk_011_break_while_if.mlog`,
`sblk_011_break_each_if.mlog`, `sblk_011_break_nearest_loop.mlog`.

### S-BLK-012 — `continue` starts the next iteration

A `continue` inside a loop body skips the REST of the body and proceeds
with the next iteration — the condition re-test for `while`, the next
item for `each`/`each_with_index`; at any nesting depth the signal
reaches the nearest loop. TW and VM behave identically.
**Anchors:** TW the `ControlFlow::ContinueLoop` propagation absorbed by
the loop arms (src/interpreter/execution.rs); VM the №658 loop-stack
fixups patched to the condition re-evaluation (`while`) or to the
increment section (`each`/`each_with_index`) — src/compiler.rs.
**Conformance:** `tests/conformance/sblk_012_continue_while_if.mlog`,
`sblk_012_continue_each_if.mlog`.

### S-BLK-013 — A trailing `let` in a value arm is Unit

A let-binding statement produces NO value (the №612 contract) — in a
value-channel arm body (a match-expression arm, an if/else expression
branch) a trailing `let` resets the arm's value to Unit; the pre-let
expression does not survive it. TW and VM behave identically.
**Anchors:** TW the `Statement::LetBinding` arm — `last_expr_value =
Unit` (src/interpreter/execution.rs); VM the value-channel let compiles
the `PushUnit` + `SetValueReg` register reset (src/compiler.rs — the
№370 register machinery).
**Conformance:** `tests/conformance/sblk_013_trailing_let_arm.mlog`.

### S-BLK-014 — A `let` in a value arm does not leak

A let inside a value-channel arm body binds in the arm's env clone (the
№14 P0-3 rule — "lets do not leak"): a shadow let never overwrites the
outer slot, the shadow does not survive the arm, and the outer name
reads unchanged after the arm — at ANY depth inside the arm (nested
blocks, loop bodies). TW and VM behave identically.
**Anchors:** TW the `env.clone()` for the value bodies (the
`Expr::BlockIfElse`/`Expr::MatchExpr` arms — src/interpreter/execution.rs);
VM the №659 fresh-slot allocation under the arm flag + the
locals/mutable restore at the arm boundary (src/compiler.rs).
**Conformance:** `tests/conformance/sblk_014_let_no_leak.mlog`,
`sblk_014_shadow_nested.mlog`.

## Honest limits (the probe findings this topic records WITHOUT norms)

The №645 probe for this topic made THREE cross-backend findings (the
№645 → №651 lineage is the precedent). The norms above exist only where
the probes agreed; all three findings are REPAIRED now — №652-a by №658
(the norms S-BLK-011/012), №652-b/№652-c by №659 (the norms
S-BLK-013/014) — and the №658/№659 probes recorded four deeper edges of
their own (below).

- **№652-a — `break`/`continue` inside a while body are ignored on the
  VM. REPAIRED by №658 (gh#1159):** the loop-stack fixups now resolve on
  BOTH backends — every break/continue (top-level of a loop body or
  nested through if/match/loop) reaches its nearest loop edge on the VM;
  the pre-№658 compiler patched the fixups from stale local vecs (a
  top-level break kept its `Jump(0)` placeholder — the infinite-loop
  guard class) and dropped the nested forms (the body ran to the
  condition cap as if the statements were absent). The norms
  S-BLK-011/012 + five conformance pairs pin the record; the TW is
  unchanged. The №658 probe also recorded TWO remaining edges (each
  needs its own repair line):
  - **№658-a — a break/continue crossing the VALUE channel.** A
    break/continue inside a match-EXPRESSION arm body (any
    expression-position body) is REFUSED by the TW at runtime
    («break/continue used outside of a loop» — the `eval_statements`
    boundary cannot carry a control signal) and is silently ABSORBED by
    the VM (the arm's value machinery stays balanced; the loop runs to
    its condition). The loud runtime mirror needs a refusal mechanism of
    its own — a separate repair naryad.
  - **№658-b — a bare break/continue OUTSIDE any loop.** The TW refuses
    at runtime with the same message; the VM compiles the statement to
    NOTHING and continues. Same repair line as №658-a.
- **№652-b — a trailing `let` in a value-channel arm diverges.
  REPAIRED by №659 (gh#1160):** the arm `{ "first" ; let s = "y" }`
  answers `Unit` on BOTH backends now — the value-channel let resets the
  №370 register (the `PushUnit` + `SetValueReg` emission, the №612
  mirror: a binding statement produces no value). The TW reading was
  chosen (the S-BLK-001-consistent «a block ending in a statement yields
  no value»); the alternative «prev-value» reading (the pre-№659 VM
  behavior) is reachable ONLY by an owner verdict — the OD note in the
  №659 naryad. The norm S-BLK-013 + the conformance pair pin the
  record.
- **№652-c — a `let` inside a VM arm leaks into the enclosing scope.
  REPAIRED by №659 (gh#1160):** the VM arm bodies now mirror the TW env
  clone (the №14 P0-3 rule) at COMPILE time — every let inside a
  value-channel arm body (at any depth: nested blocks, loop bodies)
  allocates a FRESH slot under the arm flag, and the locals/mutable maps
  are restored at the arm boundary — a shadow never overwrites the outer
  slot and does not survive the arm. The slot allocation was chosen over
  the runtime env clone (the cheaper shape — no VM-side env copying; the
  choice is recorded in the №659 report). The norm S-BLK-014 + two
  conformance pairs pin the record. The №659 probe recorded TWO deeper
  edges:
  - **№659-x — an `assign` inside a value arm.** The TW REFUSES at
    runtime (the arm's fresh mutability set does not carry the outer
    `let mut`) while the VM writes the outer slot (a probe records
    `assigned|assigned`). Out of the №659 repair letter (lets only) — a
    separate repair line.
  - **№659-y — reading an arm-local name OUTSIDE the arm.** Both
    backends refuse (the binding does not survive the arm), but with
    different codes: the TW `[UNDEFINED_VARIABLE]` at runtime, the VM a
    `[TYPE_MISMATCH]` downstream of the unresolved name — and the
    semantic checker does not model the value-arm boundary at all (the
    gh#967 §7 semantic-checker gap lane). A separate repair line.
