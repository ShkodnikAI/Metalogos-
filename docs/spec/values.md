# Topic 1 — Comparisons and truthiness (S-VAL)

**Normative.** The norms below record the REQUIRED behavior of the
comparison operators and the boolean coercion, verified identical on the
TW interpreter and the VM. The audit provenance: every statement was
already fought for by a naryad (№629 closed the silent-`false` VM class;
№371 aligned the binop wording; №479 froze the coded refusal stamp) —
this file is the RECORD, not new semantics.

## The matrix this topic encodes

| Operand class | `==` / `!=` | `>` `<` `>=` `<=` |
|---|---|---|
| Float × Float | Bool | Bool |
| String × String | Bool | refusal (`[TYPE_MISMATCH]`) |
| Bool × Bool | Bool | refusal (`[TYPE_MISMATCH]`) |
| Unit × Unit | Bool (`==` true, `!=` false) | refusal (`[TYPE_MISMATCH]`) |
| Unit × other (either side) | **Bool** (`==` false, `!=` true) | refusal (`[TYPE_MISMATCH]`) |
| other heterogeneous pairs | refusal (`[TYPE_MISMATCH]`) | refusal (`[TYPE_MISMATCH]`) |
| composites (List/Struct, incl. same-type) | refusal (`[TYPE_MISMATCH]`) | refusal (`[TYPE_MISMATCH]`) |

The boolean coercion (`if` conditions): `false`, `0.0`, the empty
`String` and `Unit` are FALSY; `true`, a non-zero `Float` and a
non-empty `String` are TRUTHY. Composites in a boolean position are NOT
part of this topic (see «Honest limits»).

## The norms

### S-VAL-001 — The falsy scalar set

An `if` condition of `false`, `0.0`, the empty `String` or `Unit` takes
the ELSE branch.
**Anchors:** TW `Value::as_bool` (src/interpreter/values.rs:421); VM
`is_truthy` (src/vm.rs:4078).
**Conformance:** `tests/conformance/sval_001_falsy.mlog` (+ `.expected`).

### S-VAL-002 — The truthy scalar set

An `if` condition of a non-empty `String`, a non-zero `Float` or `true`
takes the THEN branch.
**Anchors:** TW `Value::as_bool` (src/interpreter/values.rs:421); VM
`is_truthy` (src/vm.rs:4078).
**Conformance:** `tests/conformance/sval_002_truthy.mlog` (+ `.expected`).

### S-VAL-003 — Same-type equality answers Bool

`==` on two `String`s, two `Float`s or two `Bool`s answers `Bool` with
the natural value; no other operand class answers for `==` except the
Unit arms (S-VAL-005/006) — everything else refuses (S-VAL-007/008/012).
**Anchors:** TW `eval_binop` Eq arms (src/interpreter/execution.rs:2946);
VM `eval_cmp` Eq arms (src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_003_eq_same_type.mlog`.

### S-VAL-004 — Same-type inequality answers Bool

`!=` on two `String`s, two `Float`s or two `Bool`s answers `Bool` with
the natural value.
**Anchors:** TW `eval_binop` Ne arms (src/interpreter/execution.rs:2946);
VM `eval_cmp` Ne arms (src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_004_ne_same_type.mlog`.

### S-VAL-005 — Unit equality

`Unit == Unit` is `true`; `Unit != Unit` is `false`, on both backends.
**Anchors:** TW Eq/Ne `Unit, Unit` arms (src/interpreter/execution.rs:2946);
VM `eval_cmp` (src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_005_eq_unit.mlog`.

### S-VAL-006 — Unit-mixed equality answers Bool

`Unit == non-Unit` is `false` and `Unit != non-Unit` is `true` — the
Unit arms are EXPLICIT Bool answers, never the heterogeneous refusal
(the №629 matrix kept this arm deliberately; the refusal class starts
below, at S-VAL-007).
**Anchors:** TW `(Eq, Unit, _) | (Eq, _, Unit)` arms
(src/interpreter/execution.rs:2946); VM `eval_cmp` (src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_006_eq_unit_mixed.mlog`.

### S-VAL-007 — No silent numeric conversion

`"5" == 5.0` REFUSES with the stable `[TYPE_MISMATCH]` code on both
backends — a numeric String is never silently converted (the №629
headline case).
**Anchors:** TW `eval_binop` fall-through
(src/interpreter/execution.rs:2946); VM `eval_cmp` fall-through
(src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_007_eq_no_conversion.mlog`.

### S-VAL-008 — Heterogeneous equality refuses

`==` on operands of different non-Unit types (String/Float, Bool/Float,
String/Bool, ...) refuses with `[TYPE_MISMATCH]` on both backends.
**Anchors:** TW `eval_binop` fall-through
(src/interpreter/execution.rs:2946); VM `eval_cmp` fall-through
(src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_008_eq_heterogeneous.mlog`.

### S-VAL-009 — Heterogeneous inequality refuses

`!=` on operands of different non-Unit types refuses with
`[TYPE_MISMATCH]` on both backends (the same stable code as S-VAL-008;
the wording is not pinned by the conformance pair).
**Anchors:** TW `eval_binop` fall-through
(src/interpreter/execution.rs:2946); VM `eval_cmp` fall-through
(src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_009_ne_heterogeneous.mlog`.

### S-VAL-010 — Ordering is Float-only (the positive arm)

`>` `<` `>=` `<=` answer `Bool` for `Float × Float` pairs on both
backends.
**Anchors:** TW `eval_binop` Gt/Lt/Ge/Le arms
(src/interpreter/execution.rs:2946); VM `eval_cmp` ordering arms
(src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_010_ordering_float.mlog`.

### S-VAL-011 — Ordering refuses everything else

Ordering on non-Float pairs — Strings (`"b" > "a"`), Bools
(`true > false`), composites — refuses with `[TYPE_MISMATCH]` on both
backends. There is NO lexicographic or truth-table ordering in the
language.
**Anchors:** TW `eval_binop` fall-through
(src/interpreter/execution.rs:2946); VM `eval_cmp` fall-through
(src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_011_ordering_non_float.mlog`.

### S-VAL-012 — Composites refuse equality (incl. same-type)

`==`/`!=` on `List` or `Struct` values — INCLUDING two values of the
same composite type — refuse with `[TYPE_MISMATCH]` on both backends
(the №629 matrix: no structural equality at this stage; the opaque
values behave the same via the same fall-through).
**Anchors:** TW `eval_binop` fall-through
(src/interpreter/execution.rs:2946); VM `eval_cmp` fall-through
(src/vm.rs:4044).
**Conformance:** `tests/conformance/sval_012_eq_composites.mlog`.

## Honest limits (the audit-era findings this topic records WITHOUT norms)

- **Composites in a boolean position are a live cross-backend
  divergence** — the TW `as_bool` refuses (`cannot convert List to
  Bool`), the VM `is_truthy` answers (a non-empty List is truthy, a
  Struct is falsy). No norm records this; the divergence needs its own
  semantics naryad (the conformance runner would go red on ANY pair
  trying to pin it — that is the design working).
- **The `!=` refusal wording differs** — the VM's fall-through names the
  internal `Eq` instruction for a refused `!=` (the compiler lowers the
  operator), the TW names `Ne`. The stable code `[TYPE_MISMATCH]` is
  identical; the wording divergence is a cosmetics-class finding for a
  future naryad, deliberately outside the conformance pin.
- **The line anchors** are recorded at the №645 merge tree; the
  function names are the durable anchors, the line numbers are a
  convenience snapshot (the anchors move with honest refactors — the
  conformance pairs are the enforced contract, not the lines).
