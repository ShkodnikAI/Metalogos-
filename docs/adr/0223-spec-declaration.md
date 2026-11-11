# ADR-0223: The `spec` declaration — the code-level form of the attestation spec

**Status:** Proposed
**Date:** 2026-11-11
**Naryad:** №946 (issue #1910, Волна 97 — W-AA, №AA1)
**Depends on:** ADR-0200 (the reflex attestation certificate — the `spec_hash` consumer; W1), №826 / ADR-0216 (the metric registry, the `mutate_rollback` compare clause — the grammar twin), №181 / ADR-0117 (the distill-field vocabulary the declaration mirrors: `distill_after` / `fallback_if` / `distill_min_accuracy` / `distill_margin`), №490 (the order-free fixed field set — each field at most once, the loud parse error naming the field), №493 (the multi-form overload posture — the declaration ADDS a form, the existing surfaces untouched)
**Blocks:** план v2 §13.12 W3 («проверяемое понижение яруса» — the owner's decision 27.10, decisions 1–6 accepted verbatim, gh#1599; the roadmap `naryads/W1_reflex_attestation_and_roadmap_v2.md`)

## 1. Context

The W1 certificate machine is live: `attest_verify` computes and checks a certificate whose `spec_hash` pins the model's attestation spec — the canonical JSON (`SpecSer`, `src/nn/attest.rs:233`, the fixed field order) over the model part (`reflex_name, input_size, labels, seed, layers`) and the DECLARATION part `AttestSpecInput` (`src/nn/attest.rs:124`): `pattern_name, prompt, distill_after, fallback_if, min_accuracy, margin` (the vocabulary of the learnable pattern's distill fields, №181/ADR-0117 — `attest_input_from_learnable`, `src/builtins/attest.rs:164+`, reconstructs it from the pattern's declaration state).

The machine fact that motivates this ADR: the spec is NOT declarable in code. It is reconstructed from the mutate/distill state — a program cannot STATE "this model is attested under this spec"; the hash the certificate carries is computed from whatever the pattern's current teaching state happens to be. The W3 slot of the §13.12 line («проверяемое понижение яруса», the owner's decision 27.10, gh#1599) makes the spec a first-class program object: declared, compiler-checked, and hashed by the SAME SSOT — no second canonical form is introduced, so the declaration cannot drift from the certificates the runtime issues.

The machine facts this ADR builds on (verified on live main `499b9b6c`): the token `spec` is free in `grammar.pest` (`\bspec\b` = 0); the insertion point is the `declaration` rule (`grammar.pest:8`, sibling `reflex_decl`); `spec_decl`/`SpecDecl` have 0 occurrences across `grammar.pest`/parser/ast; the canonical builders without a model exist (`canonical_spec_from_parts`, `src/nn/attest.rs:305+`); the compare-operator vocabulary is the six canonical words (`compare_op_word` / `condition_op_word`, `src/builtins/attest.rs:183/196` — the TW/AST and VM/bytecode twins).

## 2. Decision

### 2.1 The declaration form

```
spec <IDENT> {
  pattern: IDENT,
  prompt: STRING,
  distill_after: INT,
  fallback_if: IDENT ~ compare_op ~ FLOAT,
  min_accuracy: FLOAT,
  margin: FLOAT
}
```

- **One new keyword: `spec`.** The grammar rule `spec_decl = { SPEC_KW ~ IDENT ~ LBRACE ~ spec_body ~ RBRACE }`, inserted into `declaration` as a sibling of `reflex_decl`. No other token changes; the grammar grows by exactly this rule (plus its field lines and the `SPEC_KW` token).
- **The field set is fixed, the order is free, each field at most once** (the №490 form verbatim): a repeated field is a loud PARSE error naming the field and its position — never a silent last-wins.
- **Every field is optional** — `AttestSpecInput`'s fields are `Option`s; the declaration maps them 1:1 (`pattern: IDENT` → `pattern_name: Option<String>`, `prompt: STRING` → `prompt: Option<String>`, `distill_after: INT` → `Option<usize>`, `fallback_if` → `Option<(String, f64)>` — the pair is (the canonical operator word, the threshold value), `min_accuracy: FLOAT` → `Option<f64>`, `margin: FLOAT` → `Option<f64>`). A field absent from the block is `None` — the same meaning it has in the certificate machine today.
- **AST: a `SpecDecl` node WITHOUT new `Value` variants** (the W1 rule): the declaration carries plain typed fields; nothing new enters the runtime value domain.
- **The `fallback_if` left-hand side is an IDENT validated to be `confidence`** — the one metric the attested fallback gate carries. The machine truth: `AttestSpecInput.fallback_if` stores `(operator_word, value)` — the metric NAME does not enter the record or the hash, and the doc comment pins the semantics as `confidence OP value`; the runtime fallback gate is confidence-based (`learnable.rs:691` — «fallback_if: confidence OP threshold → call LLM if condition is TRUE»). Any other IDENT is a LOUD semantic refusal naming the field — a non-confidence metric would silently alias to the same record (the same `(op, value)` pair) and forge a spec whose hash matches a different declared intent. The operator is the existing `compare_op` rule (the six operators, №826's `mutate_rollback` twin); the canonical word conversion reuses `compare_op_word`/`condition_op_word` — no second mapping table.

### 2.2 The semantic guards — loud, each naming the field

1. **Duplicate spec name** — two `spec` declarations with the same name refuse at the check (the reflex/vision duplicate-name precedent).
2. **Unknown pattern** — the `pattern:` reference must resolve to a LEARNABLE pattern declaration (the machine the spec feeds: `AttestSpecInput.pattern_name` is «the learnable pattern that distills into this model»; the sibling guards `mutate`/`adapt`/`eval` check the same learnable set, `semantic.rs:7743–7776`). Refusal text: `spec '<name>': learnable pattern '<X>' not found` (the mutate twin).
3. **Non-confidence `fallback_if` metric** — an IDENT other than `confidence` refuses: `spec '<name>': fallback_if metric '<X>' is not the runtime fallback metric — the attested fallback gate is confidence-based (AttestSpecInput: `confidence OP value`); use fallback_if: confidence <op> <value>`.
4. **Non-finite thresholds** — `min_accuracy` / `margin` / the `fallback_if` value must be finite (`NaN`/`±inf` — reachable through float-literal exponent forms) — a loud refusal naming the field.
5. **`distill_after` bounds** — the grammar's `INT` rule carries non-negativity; an out-of-`usize` literal is a loud refusal naming the field (the parse-level guard).

### 2.3 The compile contract — TW and VM, one registry shape

`SpecDecl` compiles into a VM-owned spec record: a name → `AttestSpecInput` map collected at pass-1 (the `reflex_decls` architecton — `compiler.rs:963`, the interpreter seeds its state at the same point, `execution.rs:346`; the VM processes the same declarations, `vm.rs:528`). Both backends hold the SAME record shape (the `AttestSpecInput` struct — no TW/VM divergence surface), the parity is pinned by the crosscheck corpus; a JIT gap (if one exists for declarations) is loud in the docs, never silent. Programs without a `spec` declaration compile byte-identically — the declaration ADDS capability (№493 posture).

### 2.4 The accessor `spec_of(name) -> Struct`

One new builtin `spec_of` (a `spec!()` registry row; classification: source internal, label internal — the `builtins_classification.rs` form): returns the DECLARED fields of the named spec as a Struct — the fields exactly as written (`pattern`, `prompt`, `distill_after`, `fallback_if` as the operator word + value, `min_accuracy`, `margin`), absent fields as null. **The hash is NOT part of the Struct** — the honest boundary: `spec_hash` needs the MODEL part (`reflex_name, input_size, labels, seed, layers`), which the declaration does not know; the hash is computable only after the spec is bound to a model (the contract test pins the equivalence through `canonical_spec_from_parts`). An unknown spec name refuses loudly naming the builtin.

### 2.5 The hash SSOT does not move

The declaration carries exactly the `AttestSpecInput` fields; `SpecSer`'s field order, `SPEC_METRIC = "accuracy"` (the honest current value — the metric registry does not enter the hash until a separate form fixes it), and `canonical_spec_json`/`canonical_spec_from_parts` are untouched. Every certificate issued before this declaration remains valid: the declaration is a new way to STATE a spec, not a new way to HASH one.

## 3. Alternatives considered and rejected

- **The fields on `mutate_decl`** — rejected: the spec survives mutations and belongs to the model, not to a training round; a mutate-scoped spec would change the certificate every round the teaching state changes, defeating the attestation.
- **The spec in a config file outside the code** — rejected: the compiler check of the `pattern:` reference (and of the field types) is the point of the slot; an external config would move the check outside the language and reintroduce the drift this ADR closes.
- **The `fallback_if` metric validated against the №826 metric registry** — rejected: the registry holds TRAINING metrics (`accuracy`, `macro_f1`, `min_class_recall`); the attested fallback gate is the inference-time confidence (`AttestSpecInput`'s own contract). A registry-validated name would compile to the same `(op, value)` record for different declared metrics — a hash alias across different intents. The confidence pin keeps the record one-to-one with the declared intent.

## 4. Boundaries (out of scope, sequenced by the line)

- The inclusion gate — W6; the teacher registry — W3b; the curriculum — W4; the states/hysteresis — W5 (all outside this ADR; the rule «не в ущерб §13.11 C1» holds).
- No second hash: no new canonical form over the spec, no serde-visible schema change, no `SpecSer` field added.
- The certificate is provenance/consistency, not quality (the W1 rule).

**Status:** Proposed (2026-11-11 — issued by волна 97 before the code, N1; the implementation is the sister naryad №947 and executes this document verbatim; the status pass — a separate slot of the following wave, the ADR-0182 §6 form).
